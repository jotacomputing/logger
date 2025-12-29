use std::time::{Instant, Duration};
use crossbeam::channel::Receiver;
use questdb::{ingress::{Sender,Buffer,TimestampNanos}};

use crate::logger::types::{BalanceLogs, HoldingsLogs, OrderLogs};

const FLUSH_INTERVAL : Duration = Duration::from_millis(10);
const MAX_BATCH: usize = 1000;
pub struct LogFlusher{
    pub order_log_reciver : Receiver<OrderLogs>,
    pub balance_log_receiver : Receiver<BalanceLogs>,
    pub holding_log_reciver : Receiver<HoldingsLogs>,
    pub sender              : Sender,
    pub buffer              : Buffer,
    pub last_flush          : Instant
}

impl LogFlusher{
    pub fn new(
        order_log_reciver : Receiver<OrderLogs>, 
        balance_log_receiver : Receiver<BalanceLogs>,
        holding_log_reciver : Receiver<HoldingsLogs>,
    )->Self{

        let sender_to_db = Sender::from_conf("http::addr=localhost:9000;");
        if sender_to_db.is_err(){
            eprintln!("Coundt conect to sender");
        }
        let sender = sender_to_db.unwrap();
        let buffer = sender.new_buffer();
        Self { 
            order_log_reciver, 
            balance_log_receiver, 
            holding_log_reciver, 
            sender : sender,
            buffer : buffer ,
            last_flush : Instant::now()
        }
    }

    pub fn encode_order_log(&mut self , log : OrderLogs)->questdb::Result<()>{
        self.buffer
        .table("order_logs")?
        .symbol("instrument", log.symbol.to_string())?
        .symbol("side", if log.side == 0 { "bid" } else { "ask" })?
        .symbol("event_type", match log.order_event_type {
            0 => "received",
            1 => "matched",
            2 => "canceled",
            _ => "unknown",
        })?
        .symbol("severity", match log.severity {
            0 => "info",
            1 => "error",
            2 => "debug",
            _ => "unknown",
        })?
        .symbol("source", match log.source {
            0 => "shm",
            1 => "balance_manager",
            2 => "engine",
            _ => "unknown",
        })?
        .column_i64("event_id", log.event_id as i64)?
        .column_i64("order_id", log.order_id as i64)?
        .column_i64("user_id", log.user_id as i64)?
        .column_i64("price", log.price as i64)?
        .column_i64("shares_qty", log.shares_qty as i64)?
        .at(TimestampNanos::new(log.timestamp as i64 * 1_000_000_000))?;
      Ok(())
    }

    pub fn encode_balance_log(&mut self , log : BalanceLogs)->questdb::Result<()>{
        self.buffer
        .table("balance_logs")?
        .symbol("reason", if log.reason == 0 { "lock" } else { "update" })?
        .symbol("severity", match log.severity {
            0 => "info",
            1 => "error",
            _ => "debug",
        })?
        .symbol("source", match log.source {
            0 => "shm",
            1 => "balance_manager",
            _ => "unknown",
        })?
        .column_i64("event_id", log.event_id as i64)?
        .column_i64("user_id", log.user_id as i64)?
        .column_i64("order_id", log.order_id as i64)?
        .column_i64("old_reserved_balance", log.old_reserved_balance as i64)?
        .column_i64("old_available_balance", log.old_available_balance as i64)?
        .column_i64("new_reserved_balance", log.new_reserved_balance as i64)?
        .column_i64("new_available_balance", log.new_available_balance as i64)?
        .at(TimestampNanos::new(log.timestamp as i64 * 1_000_000_000))?;
        Ok(())
    }

    pub fn encode_holding_logs(&mut self , log : HoldingsLogs)->questdb::Result<()>{
        self.buffer
        .table("holding_logs")?
        .symbol("instrument", log.symbol.to_string())?
        .symbol("reason", if log.reason == 0 { "lock" } else { "fill" })?
        .symbol("severity", match log.severity {
            0 => "info",
            1 => "error",
            _ => "debug",
        })?
        .symbol("source", match log.source {
            0 => "shm",
            1 => "balance_manager",
            _ => "unknown",
        })?
        .column_i64("event_id", log.event_id as i64)?
        .column_i64("user_id", log.user_id as i64)?
        .column_i64("order_id", log.order_id as i64)?
        .column_i64("old_reserved_holding", log.old_reserved_holding as i64)?
        .column_i64("new_reserved_holding", log.new_reserved_holding as i64)?
        .column_i64("old_available_holding", log.old_available_holding as i64)?
        .column_i64("new_available_holding", log.new_available_holding as i64)?
        .at(TimestampNanos::new(log.timestamp as i64 * 1_000_000_000))?;
        Ok(())
    }

    pub fn run (&mut self){

        loop {
            let mut did_work = false;


            for _ in 0..MAX_BATCH {
                match self.order_log_reciver.try_recv() {
                    Ok(log) => {
                        let _ = self.encode_order_log(log);
                        did_work = true;
                    }
                    Err(_) => break, // channel empty
                }
            }


            for _ in 0..MAX_BATCH {
                match self.balance_log_receiver.try_recv() {
                    Ok(log) => {
                        let _ = self.encode_balance_log(log);
                        did_work = true;
                    }
                    Err(_) => break,
                }
            }


            for _ in 0..MAX_BATCH {
                match self.holding_log_reciver.try_recv() {
                    Ok(log) => {
                        let _ = self.encode_holding_logs(log);
                        did_work = true;
                    }
                    Err(_) => break,
                }
            }

            if self.buffer.len() >= MAX_BATCH|| self.last_flush.elapsed() >= FLUSH_INTERVAL{
                if let Err(e) = self.sender.flush(&mut self.buffer) {
                    eprintln!("QuestDB flush failed: {:?}", e);
                }
                self.last_flush = Instant::now();
            }

            if !did_work{
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}