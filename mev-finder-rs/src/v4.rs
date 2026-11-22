// ==============================
// Трансформируем Solidity код в Rust
// ==============================

use alloy::sol;

// создает Rust-тип Initialize по Solidity событию Initialize (интерфейс IPoolManager.sol)
sol! {
    struct PoolKey { // ключ, по которому однозначно определяется пул в Uniswap V4
        address currency0;
        address currency1;
        uint24 fee;
        int24 tickSpacing;
        address hooks;
    }

    // Параметры swap из IPoolManager.
    struct SwapParams {
        bool zeroForOne;
        int256 amountSpecified;
        uint160 sqrtPriceLimitX96;
    }

    // Параметры test helper PoolSwapTest из v4-core.
    struct TestSettings {
        bool takeClaims;
        bool settleUsingBurn;
    }

    // ABI локального helper-контракта mev-foundry/src/ReplaySwap.sol.
    // Находится в этом блоке, потому что Alloy требует PoolKey и SwapParams
    // из того же вызова sol!.
    interface ReplaySwap {
        function swap(PoolKey key, SwapParams params, TestSettings settings, bytes hookData)
            external payable returns (int256);
    }

    event Initialize(
        bytes32 indexed id, // keccak256(abi.encode(PoolKey)) идентификатор пула. PoolKey = Solidity структура
        address indexed currency0, // token 0 address
        address indexed currency1, // token 1 address
        uint24 fee, // базовая комиссия пула
        int24 tickSpacing, // расстояние между тиками (1 тик = изменение цены на 0.01%)
        address hooks, // адрес хука, если он есть.
        uint160 sqrtPriceX96, // начальная цена пула  (представление дробного числа в Q64.96)
                            // начальную цену можно задать любую
                            // USDC имеет 6 decimals
                            // WETH имеет 18 decimals
        int24 tick // тик, который соответствует начальной цене
    );

    // создается ДО вызова afterSwap
    event Swap(
      bytes32 indexed id,
      address indexed sender,
      int128 amount0, // <0 — вход currency0 в пул, >0 — выход currency0 из пула
      int128 amount1,
      uint160 sqrtPriceX96, // цена пула ПОСЛЕ свопа
      uint128 liquidity, // активная ликвидность на текущем tick (корень из k, где k = xy). Тут сумма ликвидностей
      int24 tick, // тик после свопа
      uint24 fee // тотал комиссия в миллионных долях. 252 = 0.0252% (включая protocol fee).
  );

  #[sol(rpc)] // Rust обертка, чтобы вызывать Solidity-функцию как Rust-метод. То есть, создаем не только структуру, но еще и RPC метод
  // IStateView нужен, чтобы получить текущее состояни пула из Infura RPC и проверить, что оно не нулевое
  interface IStateView {
      function getSlot0(bytes32 poolId) // получить текущее состояние пула: цену, тик, protocolFee, lpFee
          external
          view
          returns (
              uint160 sqrtPriceX96,
              int24 tick,
              uint24 protocolFee,
              uint24 lpFee
          );

      function getLiquidity(bytes32 poolId) // получить ликвидность пула
          external
          view
          returns (uint128 liquidity);
  }

  // IV4Quoter проверяет маршрут из нескольких пулов
  struct PathKey {
      address intermediateCurrency;
      uint24 fee;
      int24 tickSpacing;
      address hooks;
      bytes hookData;
  }

  struct QuoteExactParams {
      address exactCurrency;
      PathKey[] path;
      uint128 exactAmount;
  }

  #[sol(rpc)]
  interface IV4Quoter {
      function quoteExactInput(QuoteExactParams params)
          external
          returns (uint256 amountOut, uint256 gasEstimate);
  }
}
