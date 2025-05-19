# 导出个人饭卡使用数据

## CLI

**自定义导出路径**

```sh
# 默认路径 XJTU-MEALFLOW/transactions_export.csv
cargo run -- export-csv
```

```sh
# 自定义导出路径 XJTU-MEALFLOW/my_transactions.csv
cargo run -- export-csv --output "my_transactions.csv"

# 自定义导出路径 XJTU-MEALFLOW/data/my_transactions.csv
cargo run -- export-csv --output "data/my_transactions.csv"
```

**自定义消费金额**

导出: 消费金额在 10-50 元之间的交易

```sh
# 消费金额区间 [10.00, 50.00]
cargo run -- export-csv --min-amount=10.00 --max-amount=50.00
```

**筛选指定商家交易**

> 商家名单查询详见 `XJTU-MEALFLOW/data/merchant-classification.yaml`

导出: 特定商家的交易数据, 如 “超市”

```sh
# 消费商家: 超市
cargo run -- export-csv --merchant "超市"
```

**筛选消费日期区间**

```sh
# [2022-12-09, 2024-12-09] 左闭右开
cargo run -- export-csv --time-start "2022-12-09" --time-end "2024-12-09"
# [2022-12-09, Database中最“新”的时间]
cargo run -- export-csv --time-start "2022-12-09"
# [Database中最“远古”的时间, 2024-12-09]
cargo run -- export-csv --time-end "2024-12-09"
```

**多样筛选叠加使用**

导出: 消费金额在 10-50 元 + 消费商家为“超市” 的交易

```sh
cargo run -- export-csv --min-amount=10.00 --max-amount=50.00 --merchant "超市" --output "multi_transactions.csv"
```

## GUI

WIP by bxhu...
