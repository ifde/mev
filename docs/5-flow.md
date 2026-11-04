### Общий flow: 

### Компоненты: 

1. Solidity-библиотека
- Internalization хук
- dynamic-fee хуки 
И другие зависимости: 
- V4RouteExecutor
- RouteRegistry
- ProfitDistributor
- один **DeploymentFactory** (вспомогательный Solidity смарт-контракт) 

DeploymentFactory — деплоит только наши хуки и зависимости: 

- InternalizationHook
- RouteRegistry (сюда передаем найденные с помощью Rust маршруты) 
- V4RouteExecutor

2. Rust route search

Вообще не используем DeploymentFactory. Нам тут не нужна логика хука в приниципе. 
Route Search запускается до бектеста / реального деплоя. 

Rust симулирует свопы в прокси-пуле (то же направление и amount_in) и через V4Quoter ранжирует маршруты.

Результат — routes.json:
```json
{
- reference PoolKey
- направления
- amount_in
- min_profit
- priority
- параметры target-пула (token0, token1, fee, tickSpacing, hook) 
- что был за прокси-пул
- start_block - на каком блоке искали маршруты 
}
```

3. IL-бектест

На локальном fork дважды вызываем тот же DeploymentFactory:

- baseline: hook выключен
- with_hook: настоящий hook, Router и routes.json

Затем запускаем длинный replay внешнего потока и измеряем IL.

4. Production deploy

Если бектест проходит, вызываем **Foundry Deploy script**: 
- вызывает тот же DeploymentFactory
- деплоит RouteRegistry, V4RouteExecutor и Hook через CREATE2
- инициализиует (деплоит) реальный target-пул
- вычисляет настоящий target PoolId
- подставляет его в routes.json и записывает маршруты в RouteRegistry (для этого в routes.json уже должны хранится маршуты для таргет пула).  


### Сценарий 

1. Выбираем параметры пула, которые хотим, и тип хука. 
2. Запускаем Rust инструмент для поиска маршутов (если хотим использовать InternalizationHook).  
3. Запускаем бектест, смотрим на прибыльность 
4. Деплоим 

