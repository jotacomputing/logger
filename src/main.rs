use logger::{logger::{log_flusher::LogFlusher, types::{BalanceLogWrapper, HoldingLogWrapper, OrderBookSnapShot, OrderLogWrapper, TradeLogs}}, shm::{balance_logs::BalanceLogQueue, holdings_logs::HoldingLogQueue, order_logs::OrderLogQueue, poller::LogPoller}};

fn main(){

    let (order_log_sender , order_log_receiver) = crossbeam::channel::bounded::<OrderLogWrapper>(32768);
    let (balance_log_sender , balance_log_receiver) = crossbeam::channel::bounded::<BalanceLogWrapper>(32768);
    let (holding_log_sender , holding_log_receiver) = crossbeam::channel::bounded::<HoldingLogWrapper>(32768);
    let (trade_log_sender , trade_log_receiver) = crossbeam::channel::bounded::<TradeLogs>(32768);
    let (snapshot_sender , snapshot_receiver)= crossbeam::channel::bounded::<OrderBookSnapShot>(32768);


    let poller_handle = std::thread::spawn(move||{
        core_affinity::set_for_current(core_affinity::CoreId { id: 1 });
       let mut poller = LogPoller::new(order_log_sender, balance_log_sender, holding_log_sender , trade_log_sender , snapshot_sender);
       poller.run_poller();
    });

    let flusher_handle = std::thread::spawn(move ||{
        core_affinity::set_for_current(core_affinity::CoreId { id: 4 });
        let mut flusher = LogFlusher::new(order_log_receiver, balance_log_receiver, holding_log_receiver , trade_log_receiver , snapshot_receiver);
        flusher.run();
    });

    poller_handle.join().expect("poller panicked");
    flusher_handle.join().expect("flusher panicked");
    println!("System shutdown");
}