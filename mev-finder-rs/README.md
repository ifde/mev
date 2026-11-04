### MEV-Finder - Rust инструмент для поиска прибыльных арбитражных маршрутов для интернализации MEV. 

Вход: пара токенов (token0, token1), сhain_id (ETH Mainnet), analysis_block, 


#### Начало проекта 

`cargo new --bin mev-finder-rs` - создание проекта 
Cargo - Rust package manager. Under the hood, it uses `rustc` compiler to compile source files (crates). 

Добавляем зависимости: 
``` 
cargo add alloy
cargo add anyhow
cargo add clap --features derive
cargo add tokio --features macros,rt-multi-thread 
cargo add dotenv
``` 

#### Доступные команды: 

1. Найти прокси пул для заданной пары токенов и вывести все свопы в этом пуле 
```shell 
cargo run -- list-proxy-swaps \
    --token-a 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
    --token-b 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 \
    --from-block 26310000 \
    --to-block 26312964
``` 
(3000 блоков = примерно 506 транзакций) 

#### Подключение к блокчейну 

Infurа Ethereum Mainnet RPC: https://mainnet.infura.io/v3/<YOUR-API-KEY>   
Infura Sepolia (testnet) RPC: https://sepolia.infura.io/v3/<YOUR-API-KEY>   

### Чек лист: дальшейние улучшения 
- Сделать возможность поиска прокси пулов (сейчас хардкод). Использовать SubGraph API 
- Попробовать разные сети пулов (не только Ethereum Mainnet, но и Base / Arbitrum) 
- Находить v2 и v3 пулы для свопов (вроде бы, там и так нет хуков). Использовать соответственно getAmountsOut для v2 и QuoterV2 для v3 
- В функцию pool_id_from_key передавать Rust структуру PoolKey (Создать ее) 
- Разделить проект на отедельные логические блоки. Диаграмма, как они между собой взаимодействуют. 
- Сохранять результаты все вызовов в файлы, чтобы каждый раз не перезапускать API calls с одинаковыми параметрами (предусмотреть идемпотентность) 
- Реализовать цикл из 2+ пулов (сейчас только один reference-пул). Плюс убрать хардкод amount_in при прогоне маршрута пулов 
- Доделать логику V4 Quoter 
- Правильно распределить компоненты 

Структура: 

reference_search.rs
    → GraphQL discovery
    → PoolId validation
    → StateView validation
    → Vec<ValidatedReferencePool>

  find-reference-pools
    → печатает этот Vec

  quote-two-pool-cycle
    → берёт rank=1 из этого Vec
    → строит QuoteExactParams
    → вызывает V4Quoterы


Как нужно сделать V4Quoter: 
fork до historical transaction
  → воспроизвести transaction
  → вызвать V4Quoter на локальном state после неё 
(сейчас V4Quoter вызывается на фиксированном стейте в конце блока). 

---------- 

Разделяем компоненты так:

• mev-finder-rs: получает historical swaps, находит reference-пулы и кандидаты маршрутов, сохраняет конфигурацию

• mev-foundry: выполняет настоящий последовательный IL-бектест baseline и hook-пула
• REVM напрямую пока не пишем

Это также ближе к LVR-minimizing-dynamic-fee-hook: там hook и симуляционный harness реализованы на
Solidity/Foundry. 

--------- 

• для baseline и dynamic-fee IL без internalization: долгий последовательный Foundry replay корректен
• для afterSwap с reference-pools: потребуется отдельный режим точного fork state до каждой transaction 

Нужно уметь: 
- Это будем длительный IL бектест 
- деплоить таргет пул и прогонять в нем транзакции прокси пула 
- Каждую транзакцию прокси пула прогонять на текущем состоянии таргет пула и НА СТЕЙТЕ ДО ЭТОЙ ТРАНЗАКЦИИ. То есть все равно перед каждой транзакией должен быть правильный стейт. 
- Мы сможем это сделать через Founry или нужен REVM? Как делают референс проекты? 

Следовательно, для v1 фиксируем:
  • длительный IL-бектест — Foundry
  • два target-пула, один и тот же поток proxy swaps
  • state до каждого swap получается последовательным replay
  • internalization с reference-пулами пока выключена из IL-бектеста 

Порядок реализации я рекомендую такой: 
  1. Кеш и универсальный поиск proxy-пула
  2. ProxyTransaction и выбор первых 100 по transaction_index
  3. Универсальный Route для одного reference
  4. backrun-plan.json
  5. Foundry reader и массовый запуск