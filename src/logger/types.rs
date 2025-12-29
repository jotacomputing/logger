

// order recived 
// order rejcted 
// order matched 
//  balance locked 
// balance updated 
// source 0->shm reader , 1 -> balance mangaer  , 2 -> matching engine
// severity 0-> info (from components) 1-> error , 2 -> Debug 
#[repr(C)]
#[derive( Debug, Clone, Copy )]
pub struct OrderLogs{
    pub event_id               : u64 ,
    pub order_id               : u64 ,
    pub user_id                : u64 ,
    pub price                  : u64 , 
    pub timestamp              : i64 , 
    pub symbol                 : u32 ,
    pub shares_qty             : u32 ,
    pub side                   : u8 ,
    pub order_event_type             : u8 ,   // 0 order recived at SHM reader , 1->order matched , 2 -> order canceled log 
    pub severity               : u8 , 
    pub source                 : u8 , 
} // 52 bytes  

#[derive( Debug, Clone, Copy , serde::Serialize)]
pub struct BalanceLogs{
    pub event_id               : u64 ,
    pub user_id                : u64 ,
    pub old_reserved_balance   : u64 , 
    pub old_available_balance  : u64 , 
    pub new_reserved_balance   : u64 , 
    pub new_available_balance  : u64 ,
    pub reason                 : u8 , // reso for the balance update , either balances locked = 0 , or funds updated =1
    pub order_id               : u64 , // the taker order id which caused the balance updations 
    pub timestamp              : i64 , 
    pub severity               : u8 , 
    pub source                 : u8 , 
}
// 67 bytes 


#[derive( Debug, Clone, Copy , serde::Serialize)]
pub struct HoldingsLogs{
    pub event_id                : u64 ,
    pub user_id                 : u64 ,
    pub symbol                  : u32 ,
    pub old_reserved_holding    : u32 , 
    pub new_reserved_holding    : u32 , 
    pub old_available_holding   : u32 , 
    pub new_available_holding   : u32 , 
    pub reason                  : u8 , // reso for the balance update , either hokdigs locked in sell order  = 0  , or funds updated during fills =1
    pub order_id                : u64 , // the taker order id which caused the holdigs updations 
    pub timestamp               : i64 ,
    pub severity                : u8 , 
    pub source                  : u8 , 
}
// 55 bytes 

#[derive( Debug, Clone , serde::Serialize)]
pub struct OrderBookSnapShot{
    pub event_id               : u64 ,
    pub symbol                 : u32 , 
    pub bids                   : Vec<[String ; 3]>,
    pub asks                   : Vec<[String ; 3]> ,
    pub timestamp              : i64 ,
    pub severity               : u8 , 
    pub source                 : u8 , 
}



#[derive( Debug, Clone)]
pub enum Logs{
    OrderLogs(OrderLogs),
    BalanceLogs(BalanceLogs),
    HoldingsLogs(HoldingsLogs),
}


