// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {Test} from "forge-std/Test.sol";
import {stdJson} from "forge-std/StdJson.sol";
import {IStateView} from "@uniswap/v4-periphery/src/interfaces/IStateView.sol";
import {IV4Quoter} from "@uniswap/v4-periphery/src/interfaces/IV4Quoter.sol";
import {PathKey} from "@uniswap/v4-periphery/src/libraries/PathKey.sol";
import {Currency} from "@uniswap/v4-core/src/types/Currency.sol";
import {IHooks} from "@uniswap/v4-core/src/interfaces/IHooks.sol";
import {PoolId} from "@uniswap/v4-core/src/types/PoolId.sol";

// Проверка связки Rust output.json -> Foundry fork -> V4Quoter.
// Берём первые N proxy transaction, все их маршруты и три значения amount_in для каждого.
contract EvaluationPlanQuoteTest is Test {
    using stdJson for string;

    string internal constant RESULTS_PATH =
        "test/results/evaluation-results.json";

    IStateView internal constant STATE_VIEW =
        IStateView(0x7fFE42C4a5DEeA5b0feC41C94C136Cf115597227);

    IV4Quoter internal constant V4_QUOTER =
        IV4Quoter(0x99bd42B2B22e531f283b7F8a86dFf631EB16A9D1);

    // Один quote не должен тратить неограниченное количество газа из-за hook-кода.
    uint256 internal constant MAX_QUOTER_GAS_PER_QUOTE = 1_000_000;

    bytes32 internal currentTransactionHash;
    uint256 internal currentTransactionBlock;
    uint256 internal currentTransactionIndex;
    address internal currentProxySwapInputToken;
    uint256 internal currentProxySwapAmountIn;
    address internal currentRouteInputToken;
    bool internal isFirstResult;

    function test_quotesFirstNProxyTransactions() external {
        string memory plan = vm.readFile("test/fixtures/evaluation-plan.json");
        vm.removeFile(RESULTS_PATH); // очищаем файл результата перед новым запуском
        vm.writeLine(RESULTS_PATH, "[");
        isFirstResult = true;
        uint256 maxProxyTransactions = vm.envOr(
            "MAX_PROXY_TRANSACTIONS",
            uint256(1)
        );
        assertGt(
            maxProxyTransactions,
            0,
            "MAX_PROXY_TRANSACTIONS must be positive"
        );

        uint256 proxyTransactionIndex = 0;
        uint256 nonzeroQuotes = 0;

        while (
            proxyTransactionIndex < maxProxyTransactions &&
            plan.keyExists(
                string.concat(
                    ".transaction_plans[",
                    vm.toString(proxyTransactionIndex),
                    "]"
                )
            )
        ) {
            nonzeroQuotes += _quoteProxyTransaction(
                plan,
                proxyTransactionIndex
            );
            ++proxyTransactionIndex;
        }

        emit log_named_uint(
            "processed_proxy_transaction_count",
            proxyTransactionIndex
        );
        emit log_named_uint("nonzero_quote_count", nonzeroQuotes);
        vm.writeLine(RESULTS_PATH, "]");
    }

    function _quoteProxyTransaction(
        string memory plan,
        uint256 proxyTransactionIndex
    ) internal returns (uint256 successfulQuotes) {
        // `transaction_plans` — имя массива в Rust JSON.
        // Каждый его элемент описывает одну proxy-транзакцию и её маршруты.
        string memory proxyTransactionEntryPath = string.concat(
            // get a specific transaction plan from an array of all the transaction plans
            ".transaction_plans[",
            vm.toString(proxyTransactionIndex),
            "]"
        );
        string memory transactionPath = string.concat(
            // get a transaction from this plan
            proxyTransactionEntryPath,
            ".proxy_transaction"
        );
        // what we know about this transaction
        bytes32 transactionHash = plan.readBytes32(
            string.concat(transactionPath, ".transaction_hash")
        );
        uint256 transactionBlock = plan.readUint(
            string.concat(transactionPath, ".block_number")
        );
        uint256 transactionIndex = plan.readUint(
            string.concat(transactionPath, ".transaction_index")
        );

        currentTransactionHash = transactionHash;
        currentTransactionBlock = transactionBlock;
        currentTransactionIndex = transactionIndex;
        _setCurrentProxySwapContext(plan, transactionPath);
        PoolId proxyPoolId = PoolId.wrap(
            plan.readBytes32(".proxy_pool.pool_id")
        );

        emit log_named_uint(
            "proxy_transaction_list_index",
            proxyTransactionIndex
        );
        emit log_named_uint("proxy_transaction_block", transactionBlock);
        emit log_named_uint("proxy_transaction_index", transactionIndex);
        emit log_named_bytes32("proxy_transaction_hash", transactionHash);

        vm.createSelectFork(vm.envString("MAINNET_RPC_URL"), transactionHash);

        (uint160 sqrtPriceBefore, , , ) = STATE_VIEW.getSlot0(proxyPoolId);
        vm.transact(transactionHash);
        (uint160 sqrtPriceAfter, , , ) = STATE_VIEW.getSlot0(proxyPoolId);
        assertTrue(
            sqrtPriceAfter != sqrtPriceBefore,
            "proxy transaction did not change price"
        );

        uint256 routeEvaluationIndex = 0;
        while (
            // iterate over all route plans for this transaction
            plan.keyExists(
                string.concat(
                    proxyTransactionEntryPath,
                    ".route_evaluations[",
                    vm.toString(routeEvaluationIndex),
                    "]"
                )
            )
        ) {
            successfulQuotes += _quoteRouteEvaluation(
                plan,
                proxyTransactionEntryPath,
                routeEvaluationIndex
            );
            ++routeEvaluationIndex;
        }

        emit log_named_uint("route_evaluation_count", routeEvaluationIndex);
    }

    function _quoteRouteEvaluation(
        // пробуем один маршурт (evaluation)
        string memory plan, // путь в файлу evaluation-plan.json
        string memory proxyTransactionEntryPath, // путь внутри этого JSON к плану для конкретной транзакции
        uint256 evaluationIndex // индекс для массива route evaluations
    ) internal returns (uint256 successfulQuotesForAmount) {
        string memory evaluationPath = string.concat(
            // достаем конкретный маршрут (evaluation) для проверки
            proxyTransactionEntryPath,
            ".route_evaluations[",
            vm.toString(evaluationIndex),
            "]"
        );
        // что мы знаем про этот маршрут
        uint256 routeId = plan.readUint(
            string.concat(evaluationPath, ".route_id")
        );
        uint256 amountAnchor = plan.readUint(
            string.concat(evaluationPath, ".amount_anchor")
        );
        uint256[] memory gridPpm = plan.readUintArray(
            string.concat(evaluationPath, ".grid_ppm")
        );
        string memory routePath = string.concat(
            ".routes[",
            vm.toString(routeId),
            "].route"
        );
        address startCurrency = plan.readAddress(
            string.concat(routePath, ".start_currency")
        );
        currentRouteInputToken = startCurrency;

        PathKey[] memory path = new PathKey[](2);
        path[0] = _readPathKey(plan, routePath, 0, startCurrency);
        path[1] = _readPathKey(
            plan,
            routePath,
            1,
            Currency.unwrap(path[0].intermediateCurrency)
        );
        assertEq(
            Currency.unwrap(path[1].intermediateCurrency),
            startCurrency,
            "route must end in its start currency"
        );

        if (!_routeHasActiveLiquidity(plan, routePath)) {
            emit log_named_uint("route_skipped_zero_active_liquidity", routeId);
            for (uint256 index = 0; index < gridPpm.length; ++index) {
                uint256 amountIn = (amountAnchor * gridPpm[index]) / 1_000_000;
                _writeResult(
                    routeId,
                    gridPpm[index],
                    amountIn,
                    0,
                    0,
                    amountIn == 0
                        ? "input_rounds_to_zero"
                        : "zero_active_liquidity",
                    bytes("")
                );
            }
            return 0;
        }

        emit log_named_uint("route_id", routeId);
        emit log_named_uint("route_hop_count", path.length);
        uint256 routeGasBefore = gasleft();

        for (uint256 index = 0; index < gridPpm.length; ++index) {
            uint256 amountIn = (amountAnchor * gridPpm[index]) / 1_000_000;

            if (amountIn == 0) {
                emit log_named_uint("input_rounds_to_zero", routeId);
                _writeResult(
                    routeId,
                    gridPpm[index],
                    amountIn,
                    0,
                    0,
                    "input_rounds_to_zero",
                    bytes("")
                );
                continue;
            }

            assertLe(
                amountIn,
                type(uint128).max,
                "amount_in does not fit Quoter"
            );

            emit log_named_uint("route_id", routeId);
            emit log_named_uint("grid_ppm", gridPpm[index]);

            try
                V4_QUOTER.quoteExactInput{gas: MAX_QUOTER_GAS_PER_QUOTE}(
                    IV4Quoter.QuoteExactParams({
                        exactCurrency: Currency.wrap(startCurrency),
                        path: path,
                        exactAmount: uint128(amountIn)
                    })
                )
            returns (uint256 amountOut, uint256 quoterGasEstimate) {
                emit log_named_uint("amount_in", amountIn);
                emit log_named_uint("amount_out", amountOut);
                emit log_named_int(
                    "gross_profit",
                    int256(amountOut) - int256(amountIn)
                );
                emit log_named_uint("quoter_gas_estimate", quoterGasEstimate);

                if (amountOut == 0) {
                    emit log_named_uint("quote_zero_amount_out", routeId);
                    _writeResult(
                        routeId,
                        gridPpm[index],
                        amountIn,
                        amountOut,
                        quoterGasEstimate,
                        "zero_output",
                        bytes("")
                    );
                    continue;
                }

                _writeResult(
                    routeId,
                    gridPpm[index],
                    amountIn,
                    amountOut,
                    quoterGasEstimate,
                    "ok",
                    bytes("")
                );
                ++successfulQuotesForAmount;
            } catch (bytes memory reason) {
                emit log_named_bytes("quote_revert", reason);
                _writeResult(
                    routeId,
                    gridPpm[index],
                    amountIn,
                    0,
                    0,
                    "reverted",
                    reason
                );
            }
        }

        emit log_named_uint("route_check_gas_used", routeGasBefore - gasleft());
    }

    function _readPathKey(
        string memory plan,
        string memory routePath,
        uint256 hopIndex,
        address inputCurrency
    ) internal pure returns (PathKey memory) {
        string memory hopPath = string.concat(
            routePath,
            ".hops[",
            vm.toString(hopIndex),
            "]"
        );

        address currency0 = plan.readAddress(
            string.concat(hopPath, ".currency0")
        );
        address currency1 = plan.readAddress(
            string.concat(hopPath, ".currency1")
        );
        address outputCurrency;

        if (inputCurrency == currency0) {
            outputCurrency = currency1;
        } else {
            assertEq(
                inputCurrency,
                currency1,
                "route hop does not accept input currency"
            );
            outputCurrency = currency0;
        }

        return
            PathKey({
                intermediateCurrency: Currency.wrap(outputCurrency),
                fee: uint24(plan.readUint(string.concat(hopPath, ".fee"))),
                tickSpacing: int24(
                    plan.readInt(string.concat(hopPath, ".tick_spacing"))
                ),
                hooks: IHooks(
                    plan.readAddress(string.concat(hopPath, ".hooks"))
                ),
                hookData: bytes("")
            });
    }

    function _readPoolId(
        string memory plan,
        string memory routePath,
        uint256 hopIndex
    ) internal pure returns (PoolId) {
        string memory hopPath = string.concat(
            routePath,
            ".hops[",
            vm.toString(hopIndex),
            "]"
        );

        return
            PoolId.wrap(plan.readBytes32(string.concat(hopPath, ".pool_id")));
    }

    function _routeHasActiveLiquidity(
        string memory plan,
        string memory routePath
    ) internal view returns (bool) {
        return
            STATE_VIEW.getLiquidity(_readPoolId(plan, routePath, 0)) != 0 &&
            STATE_VIEW.getLiquidity(_readPoolId(plan, routePath, 1)) != 0;
    }

    function _writeResult(
        uint256 routeId,
        uint256 gridPpm,
        uint256 amountIn,
        uint256 amountOut,
        uint256 quoterGasEstimate,
        string memory status,
        bytes memory revertData
    ) internal {
        string memory separator = isFirstResult ? "" : ",";
        vm.writeLine(
            RESULTS_PATH,
            string.concat(
                separator,
                _resultPrefix(routeId, gridPpm, amountIn),
                _resultQuote(amountIn, amountOut, quoterGasEstimate),
                _resultStatus(status, revertData)
            )
        );
        isFirstResult = false;
    }

    function _resultPrefix(
        uint256 routeId,
        uint256 gridPpm,
        uint256 amountIn
    ) internal returns (string memory) {
        return
            string.concat(
                '{"transaction_hash":"',
                vm.toString(currentTransactionHash),
                '","block_number":',
                vm.toString(currentTransactionBlock),
                ',"transaction_index":',
                vm.toString(currentTransactionIndex),
                ',"proxy_swap_input_token":"',
                vm.toString(currentProxySwapInputToken),
                '","proxy_swap_input_symbol":"',
                _tokenSymbol(currentProxySwapInputToken),
                '","proxy_swap_amount_in":',
                vm.toString(currentProxySwapAmountIn),
                ',"route_input_token":"',
                vm.toString(currentRouteInputToken),
                '","route_input_symbol":"',
                _tokenSymbol(currentRouteInputToken),
                '","route_id":',
                vm.toString(routeId),
                ',"grid_ppm":',
                vm.toString(gridPpm),
                ',"amount_in":',
                vm.toString(amountIn)
            );
    }

    function _resultQuote(
        uint256 amountIn,
        uint256 amountOut,
        uint256 quoterGasEstimate
    ) internal returns (string memory) {
        return
            string.concat(
                ',"amount_out":',
                vm.toString(amountOut),
                ',"gross_profit":',
                vm.toString(int256(amountOut) - int256(amountIn)),
                ',"quoter_gas_estimate":',
                vm.toString(quoterGasEstimate)
            );
    }

    function _resultStatus(
        string memory status,
        bytes memory revertData
    ) internal returns (string memory) {
        string memory revertDataJson = revertData.length == 0
            ? "null"
            : string.concat('"', vm.toString(revertData), '"');

        return
            string.concat(
                ',"status":"',
                status,
                '","revert_data":',
                revertDataJson,
                "}"
            );
    }

    function _setCurrentProxySwapContext(
        string memory plan,
        string memory transactionPath
    ) internal {
        bool zeroForOne = plan.readBool(
            string.concat(transactionPath, ".zero_for_one")
        );
        currentProxySwapInputToken = zeroForOne
            ? plan.readAddress(".proxy_pool.currency0")
            : plan.readAddress(".proxy_pool.currency1");
        currentProxySwapAmountIn = plan.readUint(
            string.concat(transactionPath, ".amount_in")
        );
    }

    function _tokenSymbol(address token) internal pure returns (string memory) {
        if (token == 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48) return "USDC";
        if (token == 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2) return "WETH";
        if (token == 0xdAC17F958D2ee523a2206206994597C13D831ec7) return "USDT";
        if (token == 0x6B175474E89094C44Da98b954EedeAC495271d0F) return "DAI";
        if (token == 0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599) return "WBTC";
        return "UNKNOWN";
    }
}
