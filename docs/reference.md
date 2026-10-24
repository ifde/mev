# Референсные репозитории и проекты

## MEV internalization 

- [База про MEV](https://medium.com/coinmonks/where-mev-actually-comes-from-78f9870f33bd) 

- [MEV-X Homelander — GitHub](https://github.com/mev-x-project/MEV-X-Homelander), [документация](https://mev-x-project.github.io/Homelander-docs/)

- [What The Hook](https://www.whatthehook.io/)

  Пример развёрнутого Uniswap v4 hook, который захватывает арбитраж, созданный swap, и распределяет прибыль между LP, treasury, пользователем и referral. 

- [Reflex MEV — plugin integration](https://docs.reflexmev.com/integration/plugin-based)

  Документация по захвату MEV через `afterSwap` plugin для Uniswap v4, Algebra и PancakeSwap Infinity. Полезна для минимального интерфейса хука: пороги размера сделки, `try/catch`, `minProfit` и событие `BackrunExecuted`.

- [arb-hook](https://github.com/sterlingcrispin/arb-hook)

  `afterSwap`-арбитраж: v4 target pool, V3 reference pool, flash loan и проверка фактической прибыли.

## Uniswap v4 и Solidity hooks

- [Uniswap v4 core](https://github.com/Uniswap/v4-core)

  Базовый `PoolManager`, типы пулов, права hook-ов и flash accounting. Главная Solidity-зависимость.

- [Uniswap v4 periphery](https://github.com/Uniswap/v4-periphery)

  Вспомогательные контракты, утилиты роутинга и примеры интеграции с v4.

- [Uniswap v4 template](https://github.com/Uniswap/v4-template)

  Практическая стартовая Foundry-конфигурация для нового hook-проекта.

- [Uniswap v4 hooks public](https://github.com/Uniswap/v4-hooks-public)

  Официальные production hooks от Uniswap. В частности, `StablePairHook` и `VolatilePairHook` — полезные примеры динамических комиссий, конфигурации и fork-тестов.

- [OpenZeppelin Uniswap Hooks](https://github.com/OpenZeppelin/uniswap-hooks)

  Безопасные базовые реализации: `BaseDynamicFee`, `BaseOverrideFee`, `BaseDynamicAfterFee` и примитивы custom accounting. Лучше расширять эти контракты, чем самостоятельно реализовывать permissions и проверки безопасности.

- [Uniswap Hooklist](https://github.com/Uniswap/hooklist)

  Публичный каталог развёрнутых Uniswap v4 hooks. Полезен для поиска prior art и изучения конфигураций существующих hooks.

## Rust: поиск и симуляция маршрутов

- [Degenbot](https://github.com/BowTiedDevil/degenbot)

  Главный технический референс для Rust CLI. Open-source стек для арбитража и ликвидаций с моделями V2/V3/V4, Curve и Balancer, симуляцией swap и генерацией calldata.

- [`degenbot-pathfinding`](https://docs.rs/degenbot-pathfinding/latest/degenbot_pathfinding/)

  Отдельный Rust crate для графа токенов и пулов и DFS-перебора циклов с ограничением длины. Он прямо соответствует нашей задаче: `USDC → WETH → USDC` или `USDC → DAI → WETH → USDC`.

- [`degenbot-solvers`](https://docs.rs/degenbot-solvers/latest/degenbot_solvers/)

  Математика симуляции multi-hop маршрутов и подбора входного объёма. Можно изучать или использовать для off-chain оценки маршрута. Поддержка v4 в общем поисковом pipeline пока неполна в отдельных компонентах, поэтому проект нельзя считать полностью готовым решением.

- [Alloy](https://github.com/alloy-rs/alloy)

  Основная Rust-библиотека CLI: JSON-RPC, ABI bindings, чтение состояния пулов, вызовы контрактов и конфигурационные транзакции `addRoute`.

- [Foundry](https://github.com/foundry-rs/foundry)

  `Anvil` даёт локальный fork блокчейна для Rust-симуляции маршрута и измерения gas; `forge` используется для Solidity unit-, fork- и invariant-тестов.

- [Brontes](https://github.com/SorellaLabs/brontes)

  Rust-система анализа исторического MEV. Это не готовый route finder, но сильный референс для разбора и классификации реальных арбитражных транзакций.

- [Aether](https://github.com/Pablosinyores/aether)

  Более крупный Rust MEV bot с графовым поиском, mempool backrun и симуляцией на `revm`. Полезен только как архитектурный референс: его масштаб намеренно выходит за рамки проекта.

------------ 

- [Artemis](https://github.com/paradigmxyz/artemis), [разбор](https://mteam.space/posts/how-to-use-the-artemis-mev-framework/). Супер статья! 

Rust framework с разделением для поиска MEV

- [mev-inspect-rs](https://github.com/flashbots/mev-inspect-rs)

  Исторический MEV-анализ через archive/tracing node и PostgreSQL.

- [rusty-sando](https://github.com/mouseless0x/rusty-sando)

  REVM-симуляции и подбор оптимального входного объёма. V2/V3 sandwich bot.

- [subway-rs](https://github.com/refcell/subway-rs)

  Учебный Rust bot: decode pending tx, simulation, gas accounting и bundle execution.

- [REVM + Alloy + Anvil arbitrage](https://hotpath.rs/blog/revm-alloy-anvil-arbitrage), [код](https://github.com/pawurb/univ3-revm-arbitrage). Супер статья! 

  REVM `CacheDB` для локального перебора. Anvil для fork-validation.

## Динамические комиссии и LP-логика

- [Dex-Dynamic-Fee](https://github.com/swnirk/Dex-Dynamic-Fee)

  Открытый код к статье *Dynamic Fee for Reducing Impermanent Loss in Decentralized Exchanges*. Содержит BA, DA, fixed fee и oracle-based benchmark, симулятор пользователей и исторические эксперименты. Это прямой источник для текущих dynamic-fee hooks.

- [REFLEX adaptive fee hook](https://reflexsize.agency/docs)

  Альтернативная политика динамической комиссии. Хранит состояние рынка в EMA и увеличивает комиссию для неожиданной по размеру и направлению сделки. Источник идей для `ILTrimmingFee`.

- [Spry contracts](https://github.com/SpryFinance/spry-contracts), [subgraph](https://github.com/SpryFinance/spry-subgraph)

  Full-range v4 dynamic-fee hook, CREATE2 deployment и индексатор `Initialize`/`Swap`/ticks.

## Архитектурные и исследовательские референсы

- [Angstrom](https://github.com/SorellaLabs/angstrom)

  Открытый Rust-проект Sorella Labs для альтернативной организации торговли и MEV-aware market structure. Не шаблон для нашего hook-а, но полезный пример Rust-архитектуры вокруг on-chain ликвидности и orderflow.

- [Uniswap Smart Order Router](https://github.com/Uniswap/smart-order-router)

  Роутер для выгодных пользовательских swap. Это не MEV searcher. 

- [Hookr](https://hookr.fun/)

  Пример концепции composable hook blocks: dynamic fees, LP rewards и arbitrage recapture. Конструктор 

