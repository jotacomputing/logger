use std::time::{Instant, Duration};
use crossbeam::channel::Receiver;
use questdb::{ingress::{Sender,Buffer,TimestampNanos}};

use crate::logger::types::{BalanceLogWrapper, HoldingLogWrapper, OrderLogWrapper};

const FLUSH_INTERVAL : Duration = Duration::from_millis(10);
const MAX_BATCH: usize = 1000;
pub struct LogFlusher{
    pub order_log_reciver : Receiver<OrderLogWrapper>,
    pub balance_log_receiver : Receiver<BalanceLogWrapper>,
    pub holding_log_reciver : Receiver<HoldingLogWrapper>,
    pub sender              : Sender,
    pub buffer              : Buffer,
    pub last_flush          : Instant
}

impl LogFlusher{
    pub fn new(
        order_log_reciver : Receiver<OrderLogWrapper>, 
        balance_log_receiver : Receiver<BalanceLogWrapper>,
        holding_log_reciver : Receiver<HoldingLogWrapper>,
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

    pub fn encode_order_log(&mut self , log : OrderLogWrapper)->questdb::Result<()>{
        self.buffer
        .table("order_logs")?
        .symbol("instrument", log.order_delta.symbol.to_string())?
        .symbol("side", if log.order_delta.side == 0 { "bid" } else { "ask" })?
        .symbol("event_type", match log.order_delta.order_event_type {
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
        .column_i64("event_id", log.order_delta.event_id as i64)?
        .column_i64("order_id", log.order_delta.order_id as i64)?
        .column_i64("user_id", log.order_delta.user_id as i64)?
        .column_i64("price", log.order_delta.price as i64)?
        .column_i64("shares_qty", log.order_delta.shares_qty as i64)?
        .at(TimestampNanos::new(log.timestamp as i64 * 1_000_000_000))?;
      Ok(())
    }

    pub fn encode_balance_log(&mut self , log : BalanceLogWrapper)->questdb::Result<()>{
        self.buffer
        .table("balance_logs")?
        .symbol("reason", if log.balance_delta.reason == 0 { "lock" } else { "update" })?
        .symbol("severity", match log.severity {
            0 => "info",
            1 => "error",
            _ => "debug",
        })?
        .column_i64("event_id", log.balance_delta.event_id as i64)?
        .column_i64("user_id", log.balance_delta.user_id as i64)?
        .column_i64("order_id", log.balance_delta.order_id as i64)?
        .column_i64("delta_reserved_balance", log.balance_delta.delta_reserved as i64)?
        .column_i64("delta_available_balance", log.balance_delta.delta_available as i64)?
        .at(TimestampNanos::new(log.timestamp as i64 * 1_000_000_000))?;
        Ok(())
    }

    pub fn encode_holding_logs(&mut self , log : HoldingLogWrapper)->questdb::Result<()>{
        self.buffer
        .table("holding_logs")?
        .symbol("instrument", log.holding_delta.symbol.to_string())?
        .symbol("reason", if log.holding_delta.reason == 0 { "lock" } else { "fill" })?
        .symbol("severity", match log.severity {
            0 => "info",
            1 => "error",
            _ => "debug",
        })?
        .column_i64("event_id", log.holding_delta.event_id as i64)?
        .column_i64("user_id", log.holding_delta.user_id as i64)?
        .column_i64("order_id", log.holding_delta.order_id as i64)?
        .column_i64("delta_reserved_holding", log.holding_delta.delta_reserved as i64)?
        .column_i64("delta_available_holding", log.holding_delta.delta_available as i64)?
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