use crate::{logger::types::{BalanceLogs, HoldingsLogs, OrderLogs}, shm::{balance_logs::BalanceLogQueue, holdings_logs::HoldingLogQueue, order_logs::OrderLogQueue}};
use crossbeam::channel::Sender;
pub struct LogPoller{
    pub order_log_queue   : OrderLogQueue,
    pub order_log_sender  : Sender<OrderLogs>,
    pub balance_log_queue : BalanceLogQueue,
    pub balance_log_sender : Sender<BalanceLogs>,
    pub holding_log_queue : HoldingLogQueue,
    pub holding_log_sender : Sender<HoldingsLogs>,

}

impl LogPoller{
    pub fn new(order_log_sender  : Sender<OrderLogs> , 
        balance_log_sender : Sender<BalanceLogs>,
        holding_log_sender : Sender<HoldingsLogs>
    )->Self{
        let order_log_shm_queue = OrderLogQueue::open("/tmp/OrderLogs");
        let balance_log_shm_queue = BalanceLogQueue::open("/tmp/BalanceLogs");
        let holdings_log_queue = HoldingLogQueue::open("/tmp/HoldingLogs");
        if order_log_shm_queue.is_err(){
            eprintln!("failed to open the log queue");
        }
        if balance_log_shm_queue.is_err(){
            eprintln!("failed to open the log queue");
        }
        if holdings_log_queue.is_err(){
            eprintln!("failed to open the log queue");
        }

        Self { 
            order_log_queue: order_log_shm_queue.unwrap(), 
            order_log_sender,
            balance_log_queue: balance_log_shm_queue.unwrap(), 
            balance_log_sender,
            holding_log_queue: holdings_log_queue.unwrap(),
            holding_log_sender
        }
    }

    pub fn run_poller(&mut self){
        loop {
            if let Ok(Some(balance_log))=self.balance_log_queue.dequeue(){
                let _ = self.balance_log_sender.try_send(balance_log);
            }
            if let Ok(Some(holding_log))=self.holding_log_queue.dequeue(){
                let _ = self.holding_log_sender.try_send(holding_log);
            }
            if let Ok(Some(order_log))=self.order_log_queue.dequeue(){
                let _ = self.order_log_sender.try_send(order_log);
            }
        }
    }
}