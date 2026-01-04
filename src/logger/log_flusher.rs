use std::time::{Instant, Duration};
use crossbeam::channel::Receiver;
use questdb::ingress::{Sender, Buffer, TimestampNanos};

use crate::logger::types::{
    BalanceLogWrapper,
    HoldingLogWrapper,
    OrderBookSnapShot,
    OrderLogWrapper,
    TradeLogs,
};

const FLUSH_INTERVAL: Duration = Duration::from_millis(10);

const TRADE_BATCH: usize = 256;
const ORDER_BATCH: usize = 256;
const BALANCE_BATCH: usize = 256;
const HOLDING_BATCH: usize = 256;

pub struct LogFlusher {
    pub order_log_reciver: Receiver<OrderLogWrapper>,
    pub balance_log_receiver: Receiver<BalanceLogWrapper>,
    pub holding_log_reciver: Receiver<HoldingLogWrapper>,
    pub trade_log_reciver: Receiver<TradeLogs>,
    pub snapshot_reciver: Receiver<OrderBookSnapShot>,

    pub sender: Sender,
    pub buffer: Buffer,

    pub rows_written: usize,
    pub last_flush: Instant,
}

impl LogFlusher {
    pub fn new(
        order_log_reciver: Receiver<OrderLogWrapper>,
        balance_log_receiver: Receiver<BalanceLogWrapper>,
        holding_log_reciver: Receiver<HoldingLogWrapper>,
        trade_log_reciver: Receiver<TradeLogs>,
        snapshot_reciver: Receiver<OrderBookSnapShot>,
    ) -> Self {
        let sender = Sender::from_conf("http::addr=localhost:9000;")
            .expect("Failed to connect to QuestDB");

        let buffer = sender.new_buffer();

        Self {
            order_log_reciver,
            balance_log_receiver,
            holding_log_reciver,
            trade_log_reciver,
            snapshot_reciver,
            sender,
            buffer,
            rows_written: 0,
            last_flush: Instant::now(),
        }
    }

   

    #[inline(always)]
    fn encode_snapshot(&mut self, snap: OrderBookSnapShot) -> questdb::Result<()> {
        let bids_json = serde_json::to_string(&snap.bids).unwrap();
        let asks_json = serde_json::to_string(&snap.asks).unwrap();

        self.buffer
            .table("orderbook_snapshots")?
            .symbol("symbol", snap.symbol.to_string())?
            .column_i64("snapshot_id", snap.event_id as i64)?
            .column_str("bids", &bids_json)?
            .column_str("asks", &asks_json)?
            .at(TimestampNanos::new(snap.timestamp))?;

        // 🔥 Snapshot correctness > throughput
        self.sender.flush(&mut self.buffer)?;
        self.buffer = self.sender.new_buffer();

        Ok(())
    }

   

    #[inline(always)]
    fn encode_order_log(&mut self, log: OrderLogWrapper) -> questdb::Result<()> {
        self.buffer
            .table("order_logs")?
            .symbol("instrument", log.order_delta.symbol.to_string())?
            .symbol("side", if log.order_delta.side == 0 { "bid" } else { "ask" })?
            .symbol(
                "event_type",
                match log.order_delta.order_event_type {
                    0 => "received",
                    1 => "matched",
                    2 => "canceled",
                    _ => "unknown",
                },
            )?
            .symbol(
                "severity",
                match log.severity {
                    0 => "info",
                    1 => "error",
                    2 => "debug",
                    _ => "unknown",
                },
            )?
            .column_i64("event_id", log.order_delta.event_id as i64)?
            .column_i64("order_id", log.order_delta.order_id as i64)?
            .column_i64("user_id", log.order_delta.user_id as i64)?
            .column_i64("price", log.order_delta.price as i64)?
            .column_i64("shares_qty", log.order_delta.shares_qty as i64)?
            .at(TimestampNanos::new(log.timestamp))?;

        self.rows_written += 1;
        Ok(())
    }

    #[inline(always)]
    fn encode_balance_log(&mut self, log: BalanceLogWrapper) -> questdb::Result<()> {
        self.buffer
            .table("balance_logs")?
            .symbol("reason", if log.balance_delta.reason == 0 { "lock" } else { "update" })?
            .symbol("severity", "info")?
            .column_i64("event_id", log.balance_delta.event_id as i64)?
            .column_i64("user_id", log.balance_delta.user_id as i64)?
            .column_i64("order_id", log.balance_delta.order_id as i64)?
            .column_i64("delta_reserved_balance", log.balance_delta.delta_reserved as i64)?
            .column_i64("delta_available_balance", log.balance_delta.delta_available as i64)?
            .at(TimestampNanos::new(log.timestamp))?;

        self.rows_written += 1;
        Ok(())
    }

    #[inline(always)]
    fn encode_holding_log(&mut self, log: HoldingLogWrapper) -> questdb::Result<()> {
        self.buffer
            .table("holding_logs")?
            .symbol("instrument", log.holding_delta.symbol.to_string())?
            .symbol("reason", "update")?
            .symbol("severity", "info")?
            .column_i64("event_id", log.holding_delta.event_id as i64)?
            .column_i64("user_id", log.holding_delta.user_id as i64)?
            .column_i64("order_id", log.holding_delta.order_id as i64)?
            .column_i64("delta_reserved_holding", log.holding_delta.delta_reserved as i64)?
            .column_i64("delta_available_holding", log.holding_delta.delta_available as i64)?
            .at(TimestampNanos::new(log.timestamp))?;

        self.rows_written += 1;
        Ok(())
    }

    #[inline(always)]
    fn encode_trade_log(&mut self, log: TradeLogs) -> questdb::Result<()> {
        self.buffer
            .table("trade_logs")?
            .symbol("symbol", log.symbol.to_string())?
            .column_i64("price", log.price as i64)?
            .column_i64("quantity", log.quantity as i64)?
            .column_i64("buyer_order_id", log.buyer_order_id as i64)?
            .column_i64("seller_order_id", log.seller_order_id as i64)?
            .column_bool("is_buyer_maker", log.is_buyer_maker)?
            .at(TimestampNanos::new(log.timestamp))?;

        self.rows_written += 1;
        Ok(())
    }

    fn try_flush(&mut self) {
        if self.rows_written == 0 {
            return;
        }

        if self.last_flush.elapsed() >= FLUSH_INTERVAL {
            let _ = self.sender.flush(&mut self.buffer);
            self.buffer = self.sender.new_buffer();
            self.rows_written = 0;
            self.last_flush = Instant::now();
        }
    }

   

    pub fn run(&mut self) {
        loop {
            let mut did_work = false;

           
            while let Ok(snap) = self.snapshot_reciver.try_recv() {
                let _ = self.encode_snapshot(snap);
                did_work = true;
            }

            for _ in 0..TRADE_BATCH {
                if let Ok(log) = self.trade_log_reciver.try_recv() {
                    let _ = self.encode_trade_log(log);
                    did_work = true;
                } else { break; }
            }

            for _ in 0..ORDER_BATCH {
                if let Ok(log) = self.order_log_reciver.try_recv() {
                    let _ = self.encode_order_log(log);
                    did_work = true;
                } else { break; }
            }

            for _ in 0..BALANCE_BATCH {
                if let Ok(log) = self.balance_log_receiver.try_recv() {
                    let _ = self.encode_balance_log(log);
                    did_work = true;
                } else { break; }
            }

            for _ in 0..HOLDING_BATCH {
                if let Ok(log) = self.holding_log_reciver.try_recv() {
                    let _ = self.encode_holding_log(log);
                    did_work = true;
                } else { break; }
            }

            self.try_flush();

            if !did_work {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}
