// ==============================
// КОНСТАНТЫ
// ============================== 

// для модуля proxy_transactions.rs 
pub const MAX_LOG_BLOCKS: u64 = 10_000; // максимально возможное число блоков, которое достаем за один раз из eth_getLogs
pub const MAX_PROXY_TRANSACTIONS: usize = 100; // сколько внешних транзакций передаём в Foundry

// сколько максимально пулов ищем, через которые будем делать свопы
pub const MAX_REFERENCE_POOL_CANDIDATES: usize = 16; 
pub const MAX_PROXY_POOL_CANDIDATES: usize = 16;

