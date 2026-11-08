### Наблюдения 

#### 1. Чем меньше процент input, тем более прибыльная транзакция 

Параметры запуска: 
```shell 
cargo run -- build-evaluation-plan \
    --analysis-block 26319254 \
    --token-a 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
    --token-b 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 \
    --from-block 26290000 \
    --to-block 26319254 \
    --output ../mev-foundry/test/fixtures/evaluation-plan.json

MAX_PROXY_TRANSACTIONS=3 forge test --match-contract EvaluationPlanQuoteTest -vv
```

Результат: 

```json 
,{"transaction_hash":"0x3d12817a376a6adf4eea853ed1d2aff6502d12f4190bd1c251a39ccf8a959cb6","block_number":26290549,"transaction_index":2,"route_id":4,"grid_bps":100,"amount_in":12028,"amount_out":11926,"gross_profit":-102,"quoter_gas_estimate":84777,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3d12817a376a6adf4eea853ed1d2aff6502d12f4190bd1c251a39ccf8a959cb6","block_number":26290549,"transaction_index":2,"route_id":4,"grid_bps":2050,"amount_in":246587,"amount_out":217634,"gross_profit":-28953,"quoter_gas_estimate":106337,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3d12817a376a6adf4eea853ed1d2aff6502d12f4190bd1c251a39ccf8a959cb6","block_number":26290549,"transaction_index":2,"route_id":4,"grid_bps":4000,"amount_in":481146,"amount_out":413893,"gross_profit":-67253,"quoter_gas_estimate":106305,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3d12817a376a6adf4eea853ed1d2aff6502d12f4190bd1c251a39ccf8a959cb6","block_number":26290549,"transaction_index":2,"route_id":7,"grid_bps":100,"amount_in":96807694255400,"amount_out":85778831332529,"gross_profit":-11028862922871,"quoter_gas_estimate":106373,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3d12817a376a6adf4eea853ed1d2aff6502d12f4190bd1c251a39ccf8a959cb6","block_number":26290549,"transaction_index":2,"route_id":7,"grid_bps":2050,"amount_in":1984557732235717,"amount_out":1630291089956026,"gross_profit":-354266642279691,"quoter_gas_estimate":105516,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3d12817a376a6adf4eea853ed1d2aff6502d12f4190bd1c251a39ccf8a959cb6","block_number":26290549,"transaction_index":2,"route_id":7,"grid_bps":4000,"amount_in":3872307770216034,"amount_out":3103626615784260,"gross_profit":-768681154431774,"quoter_gas_estimate":106589,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3268ee4c08ee9c2b38355e59c83d41cd947fc1d6ebba08feca2e06fcc28fd42e","block_number":26319087,"transaction_index":14,"route_id":0,"grid_bps":100,"amount_in":708,"amount_out":638,"gross_profit":-70,"quoter_gas_estimate":66944,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3268ee4c08ee9c2b38355e59c83d41cd947fc1d6ebba08feca2e06fcc28fd42e","block_number":26319087,"transaction_index":14,"route_id":0,"grid_bps":2050,"amount_in":14525,"amount_out":12999,"gross_profit":-1526,"quoter_gas_estimate":72133,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3268ee4c08ee9c2b38355e59c83d41cd947fc1d6ebba08feca2e06fcc28fd42e","block_number":26319087,"transaction_index":14,"route_id":0,"grid_bps":4000,"amount_in":28342,"amount_out":25147,"gross_profit":-3195,"quoter_gas_estimate":72105,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3268ee4c08ee9c2b38355e59c83d41cd947fc1d6ebba08feca2e06fcc28fd42e","block_number":26319087,"transaction_index":14,"route_id":3,"grid_bps":100,"amount_in":298805127834,"amount_out":269126362822,"gross_profit":-29678765012,"quoter_gas_estimate":66155,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3268ee4c08ee9c2b38355e59c83d41cd947fc1d6ebba08feca2e06fcc28fd42e","block_number":26319087,"transaction_index":14,"route_id":3,"grid_bps":2050,"amount_in":6125505120611,"amount_out":5484403919251,"gross_profit":-641101201360,"quoter_gas_estimate":72165,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3268ee4c08ee9c2b38355e59c83d41cd947fc1d6ebba08feca2e06fcc28fd42e","block_number":26319087,"transaction_index":14,"route_id":3,"grid_bps":4000,"amount_in":11952205113387,"amount_out":10615086785068,"gross_profit":-1337118328319,"quoter_gas_estimate":72133,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3268ee4c08ee9c2b38355e59c83d41cd947fc1d6ebba08feca2e06fcc28fd42e","block_number":26319087,"transaction_index":14,"route_id":4,"grid_bps":100,"amount_in":708,"amount_out":645,"gross_profit":-63,"quoter_gas_estimate":68821,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3268ee4c08ee9c2b38355e59c83d41cd947fc1d6ebba08feca2e06fcc28fd42e","block_number":26319087,"transaction_index":14,"route_id":4,"grid_bps":2050,"amount_in":14525,"amount_out":13142,"gross_profit":-1383,"quoter_gas_estimate":84641,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3268ee4c08ee9c2b38355e59c83d41cd947fc1d6ebba08feca2e06fcc28fd42e","block_number":26319087,"transaction_index":14,"route_id":4,"grid_bps":4000,"amount_in":28342,"amount_out":25422,"gross_profit":-2920,"quoter_gas_estimate":84613,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3268ee4c08ee9c2b38355e59c83d41cd947fc1d6ebba08feca2e06fcc28fd42e","block_number":26319087,"transaction_index":14,"route_id":7,"grid_bps":100,"amount_in":298805127834,"amount_out":272357732102,"gross_profit":-26447395732,"quoter_gas_estimate":68032,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3268ee4c08ee9c2b38355e59c83d41cd947fc1d6ebba08feca2e06fcc28fd42e","block_number":26319087,"transaction_index":14,"route_id":7,"grid_bps":2050,"amount_in":6125505120611,"amount_out":5544397291280,"gross_profit":-581107829331,"quoter_gas_estimate":84673,"status":"ok","revert_data":null}
,{"transaction_hash":"0x3268ee4c08ee9c2b38355e59c83d41cd947fc1d6ebba08feca2e06fcc28fd42e","block_number":26319087,"transaction_index":14,"route_id":7,"grid_bps":4000,"amount_in":11952205113387,"amount_out":10729607129579,"gross_profit":-1222597983808,"quoter_gas_estimate":84645,"status":"ok","revert_data":null}
]
``` 

Доля proxy swap    amount_in, raw USDC    amount_out, raw USDC     gross_profit
  ━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━
   0.5%                               354                     339              -15
  ─────────────────  ─────────────────────  ──────────────────────  ───────────────
   0.1%                                70                      67               -3
  ─────────────────  ─────────────────────  ──────────────────────  ───────────────
   0.005%                               3                       1               -2
  ─────────────────  ─────────────────────  ──────────────────────  ───────────────
   0.001%                               0                       —    не существует


