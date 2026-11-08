### Общие заметки 

#### SubGraph 

Позволяет делать SQL запросы к Uniswap V4 пулам, чтобы узнать их текущую ликвидность / цену / состояние. 

Ссылка: https://thegraph.com/explorer/subgraphs/8B2wKxnkciCTc5HSgsAojF6vhKn6wxQ1nVecYzMge1hA?view=Query&chain=arbitrum-one 

#### Конкретные пулы UniswapV4 

Можно еще посмотреть в официальном дашборде от Uniswap: 

https://app.uniswap.org/explore/pools/ethereum/0x5a5c7cab5f55c7ea020e97d4fa6dd5d99270e56ce76afa61d8cbddec0af92060?chart=depth 

#### Infura 

URL: https://mainnet.infura.io/v3/<YOUR-API-KEY>
Через нее можно делать вызовы eth_call() 

С помощью этого получаем транзакции прокси-пула 

Подключение к Sepolia: https://sepolia.infura.io/v3/<YOUR-API-KEY>   

### Q64.96 в UniswapV4 

`uint160 sqrtPriceX96` - начальная цена пула  (представление дробного числа в Q64.96)
То есть, $sqrtPriceX96 = \sqrt{P} \cdot 2^{96}$ (unit160 хранит двоичное число с 160 битами) 

### 1 блок - это много? 

Новый блок создается раз в 12 секунд. В 1 блоке примерно от 250 до 600 транзакций. Ссылка 

1 час = 60 * 60 / 12 = 300 блоков 

10_000 блоков = 33 часа (условно - 1.5 суток) 

#### Пулы, где много токенов, но ликвидность = 0 

Получается, есть пулы где много токенов но просто никто не ставит ликвидность в текущие тики 

Хотя на самом деле ликвидность там маленькая 

Вот пример: https://app.uniswap.org/explore/pools/ethereum/0xf92118dfd8a825604e56784d4387aea38559cc78e8fab5d395533cb2e37ce353?chart=depth 

#### Общая логика пулов, позиций и ликвидности 

##### Создание пула ликвидности: 

В v4 пул определяется через `PoolKey = (currency0, currency1, fee, tickSpacing, hooks)`. У пула нет собственного адреса контракта, его состояние хранит общий `PoolManager`. 
Почему вообще при создании пула задается tickSpacing? LP хочет задать, чтобы его ликвидность (право использовать его токены в свопах) была активна в определенной границе цен. При выходе за эти границы LP перестает получать комиссии. tickSpacing задает, насколько точно можно конфигурировать это price range. 
tickSpacing = 1 означает: при каждом изменении цены на 0.01% будет включаться / выключаться ликвидность позиций (если через этот тик проходит граница позиции) => тратим больше газа на это, зато range ликвидности можно задавать более точно. Подходит для stable-coins.   
tickSpacing = 60 означает: меньше точность размещения ликвидности, но и меньше потенциальных пересечений границ тиков => меньше gas swap. 
Документация [Uniswap V4](https://developers.uniswap.org/docs/get-started/concepts/liquidity-providers/concentrated-liquidity) 

##### Initialize

Вызов `PoolManager.initialize(PoolKey, sqrtPriceX96)` создаёт состояние пула в общем `PoolManager`. Он не вносит токены и не создаёт ликвидность: только фиксирует стартовую цену и начальный tick.

Событие `Initialize` содержит `PoolId`, `currency0`, `currency1`, `fee`, `tickSpacing`, адрес `hooks`, `sqrtPriceX96` и начальный `tick`. По этим полям можно восстановить `PoolKey` и проверить `PoolId`.

##### Создание позиции ликвидности

Отдельной операции `createPosition` в v4 нет. Позиция неявно появляется при первом вызове `modifyLiquidity` с `liquidityDelta > 0`.

Внутри `PoolManager.unlock` вызываем `modifyLiquidity(PoolKey, ModifyLiquidityParams, hookData)`. В `ModifyLiquidityParams` передаём `tickLower`, `tickUpper`, положительный `liquidityDelta` и `salt`. 

Если диапазон тиков содержит текущую цену, PoolManager рассчитывает, сколько каждого токена нужно добавить токенов, чтобы цена не изменилась. 
Если диапазон целиком выше или ниже текущей цены, нужен только один токен 

ИЗМЕНЕНИЕ ЛИКВИДНОСТИ НЕ ДВИГАЕТ ЦЕНУ. Цену двигает только Swap. 

Позиция однозначно определяется `PoolId`, адресом владельца, диапазоном `[tickLower, tickUpper]` и `salt`. После вызова нужно оплатить дельты токенов через `settle`.

##### Добавление ликвидности

Для увеличения существующей позиции снова вызываем `modifyLiquidity` с теми же `PoolKey`, `tickLower`, `tickUpper` и `salt`, но с новым положительным `liquidityDelta`.

Другой диапазон или другой `salt` создаёт другую позицию, а не увеличивает прежнюю. Если текущая цена вне `[tickLower, tickUpper]`, добавленные токены остаются в позиции, но не участвуют в swap до возврата цены в диапазон.

Своп: 

Swap event определяет amount0 и amount1 как **изменение баланса самого PoolManager**, не
пользователя.

(amount0 > 0, amount1 < 0) => PoolManager получил currency0 и отдал currency1 
zeroForOne = true

amount0 < 0, amount1 > 0 => PoolManager получил currency1 и отдал currency0
zeroForOne = false 

#### BPS, PPM 

PPM = Parts Per Million = 1 / 1_000_000 = 1 / 10_000 % 

BPS = Basis Point = 1 / 10_000 = 1 / 100 % 

