# BMS BLE tool

[![github](https://img.shields.io/badge/github-katyo/ubmsc--rs-8da0cb.svg?style=for-the-badge&logo=github)](https://github.com/katyo/ubmsc-rs)
[![crate](https://img.shields.io/crates/v/ubmsc.svg?style=for-the-badge&color=fc8d62&logo=rust)](https://crates.io/crates/ubmsc)
[![docs](https://img.shields.io/badge/docs.rs-ubmsc-66c2a5?style=for-the-badge&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K)](https://docs.rs/ubmsc)
[![MIT](https://img.shields.io/badge/License-MIT-brightgreen.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![CI](https://img.shields.io/github/actions/workflow/status/katyo/ubmsc-rs/ci.yml?branch=master&style=for-the-badge&logo=github-actions&logoColor=white)](https://github.com/katyo/ubmsc-rs/actions?query=workflow%3ARust)

This is a tool for interacting with some customer-grade Battery Management
Systems (BMS).

## Inplemented features

- Discovering compatible BLE devices
- Identifying devices by address or name
- Querying device info
- Fetching cells data
- Command-line interface
- Prometheus exporter
- Prometheus push gateway client

## Supported models and firmware versions

| Vendor     | Model    | HW Version | SW Version |
|------------|----------|------------|------------|
| JiKong BMS | BD4A8S4P | 15A        | 15.26      |

I'm happy to add support for other models in the future.
Please open PR or create issue.

## Crate usage examples

Get device info and cells data by device name:
```rust,no_run
use btleplug::{api::Manager as _, platform::Manager};
use ubmsc::Client;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager::new().await?;

    let adapter = manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or("No adapters found")?;

    let client = Client::new(&adapter, &"UPS_BMS".into(), &Default::default());

    let info = client.device_info().await?;
    println!("{info:?}");

    let data = client.cell_data().await?;
    println!("{data:?}");

    client.close().await?;

    Ok(())
}
```

Find available BMS devices and print device info:
```rust,no_run
use btleplug::{api::Manager as _, platform::Manager};
use ubmsc::Client;

#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager::new().await?;

    let adapter = manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or("No adapters found")?;

    for device in Client::find(&adapter, &Default::default()).await? {
        let client = Client::new(&adapter, &"UPS_BMS".into(), &Default::default());

        let name = client.device_name().await?;
        let info = client.device_info().await?;
        println!("{name}");
        println!("{info:?}");

        client.close().await?;
    }

    Ok(())
}
```

## Command-line usage examples

Show help:
```plain
$ ubmsc --help`
```
```plain
Usage: ubmsc [-v] [-l <filter>] [-j] [-t <seconds>] [-r <seconds>] [-d <address...>]
             [-f <format>] [-i] [-c] [-e] [-p] [-u <url>] [-s <seconds>]

Battery Management Systems (BMS) interface.

When passed both -e and -p options push client will be run in continuous mode with specified interval.

Options:
  -v, --version                    Show version and exit.
  -l, --log <filter>               Logging filter (example: jk_bms=debug)
  -j, --journal                    Enable log to journald (log to stderr by default)
  -t, --scan-timeout <seconds>     Bluetooth scanning timeout in seconds (30 by default)
  -r, --request-timeout <seconds>  Bluetooth request timeout in seconds (5 by default)
  -d, --devices <address>          Device addresses or names (will try to scan if nothing passed)
  -f, --format <format>            Data format: rust(r) (by default) rust-pretty(R) json(j)
                                   json-pretty(J) yaml(y) toml(t) toml-pretty(T) metrics(m)
  -i, --device-info                Show device info
  -c, --cell-data                  Show cell data
  -e, --exporter                   Run prometheus exporter
  -p, --push                       Run prometheus push gateway client
  -u, --url <url>                  Prometheus exporter URL to listen/connect
  -s, --scrape-interval <seconds>  Metrics scraping interval (60s by default)
  -h, --help                       Show this help message and exit.
```

Get device info by device name and output in JSON format:
```plain
$ ubmsc -f J -i -d UPS_BMS
```
```json
{
  "device_info": [
    {
      "device_model": "JK_BD4A8S4P",
      "hardware_version": "15A",
      "software_version": "15.26",
      "up_time": 1709100,
      "poweron_times": 1,
      "device_name": "UPS_BMS",
      "device_passcode": "1234",
      "manufacturing_date": "240818",
      "serial_number": "40531310629",
      "passcode": "000",
      "userdata": "JK-BMS",
      "setup_passcode": "123456789",
      "userdata2": "JK-BMS"
    }
  ]
}
```

Scan BLE to find devices and print device info and cells data in TOML format:
```plain
$ ubmsc -f t -i -c
```
```toml
[[device_info]]
device_model = "JK_BD4A8S4P"
hardware_version = "15A"
software_version = "15.26"
up_time = 1707500
poweron_times = 1
device_name = "UPS_BMS"
device_passcode = "1234"
manufacturing_date = "240818"
serial_number = "40531310629"
passcode = "000"
userdata = "JK-BMS"
setup_passcode = "123456789"
userdata2 = "JK-BMS"
[[cell_data]]
cell_voltage = [2.384000062942505, 2.384000062942505, 2.383000135421753, 2.384000062942505, 2.384000062942505, 2.384000062942505]
average_cell_voltage = 2.384000062942505
delta_cell_voltage = 0.0010000000474974513
balance_current = 1.0240000486373901
cell_resistance = [0.1380000114440918, 0.13700000941753387, 0.14000000059604645, 0.1380000114440918, 0.13900001347064972, 0.13900001347064972]
battery_voltage = 14.304000854492188
battery_power = 2.2310001850128174
battery_current = 0.15600000321865082
battery_temperature = [23.200000762939453, 23.600000381469727]
mosfet_temperature = 25.399999618530273
remain_percent = 100
remain_capacity = 12.000000953674316
nominal_capacity = 12.000000953674316
cycle_count = 1
cycle_capacity = 18.464000701904297
up_time = 1707600
```

Show BMS cell data in Prometheus metrics format:
```plain
$ ubmsc -f metrics -c -d UPS_BMS
```
```shell
# HELP average_cell_voltage Average voltage of cells, V
# TYPE average_cell_voltage gauge
average_cell_voltage{device="UPS_BMS"} 2.385000228881836
# HELP balance_current Cells balance current, A
# TYPE balance_current gauge
balance_current{device="UPS_BMS"} 0
# HELP battery_current Current of battery, A
# TYPE battery_current gauge
battery_current{device="UPS_BMS"} 0.07800000160932541
# HELP battery_power Power of battery, W
# TYPE battery_power gauge
battery_power{device="UPS_BMS"} 1.1160000562667847
# HELP battery_temperature Temperatures of battery, ℃
# TYPE battery_temperature gauge
battery_temperature{cell="0",device="UPS_BMS"} 22.80000114440918
battery_temperature{cell="1",device="UPS_BMS"} 23.200000762939453
# HELP battery_voltage Voltage of battery, V
# TYPE battery_voltage gauge
battery_voltage{device="UPS_BMS"} 14.308000564575195
# HELP cell_resistance Resistances of cells, Ω
# TYPE cell_resistance gauge
cell_resistance{cell="0",device="UPS_BMS"} 0.1380000114440918
cell_resistance{cell="1",device="UPS_BMS"} 0.13700000941753387
cell_resistance{cell="2",device="UPS_BMS"} 0.14000000059604645
cell_resistance{cell="3",device="UPS_BMS"} 0.1380000114440918
cell_resistance{cell="4",device="UPS_BMS"} 0.13900001347064972
cell_resistance{cell="5",device="UPS_BMS"} 0.13900001347064972
# HELP cell_voltage Voltages of cells, V
# TYPE cell_voltage gauge
cell_voltage{cell="0",device="UPS_BMS"} 2.386000156402588
cell_voltage{cell="1",device="UPS_BMS"} 2.384000062942505
cell_voltage{cell="2",device="UPS_BMS"} 2.384000062942505
cell_voltage{cell="3",device="UPS_BMS"} 2.384000062942505
cell_voltage{cell="4",device="UPS_BMS"} 2.384000062942505
cell_voltage{cell="5",device="UPS_BMS"} 2.384000062942505
# HELP cycle_capacity Cycle capacity, A·h
# TYPE cycle_capacity counter
cycle_capacity{device="UPS_BMS"} 19.117000579833984
# HELP cycle_count Number of battery cicles
# TYPE cycle_count counter
cycle_count{device="UPS_BMS"} 1
# HELP delta_cell_voltage Delta voltage of cells, V
# TYPE delta_cell_voltage gauge
delta_cell_voltage{device="UPS_BMS"} 0
# HELP mosfet_temperature Temperature of mosfet, ℃
# TYPE mosfet_temperature gauge
mosfet_temperature{device="UPS_BMS"} 24.899999618530273
# HELP poweron_times Number of poweron cicles
# TYPE poweron_times counter
poweron_times{device="UPS_BMS"} 1
# HELP remain_capacity Remain capacity of battery, A·h
# TYPE remain_capacity gauge
remain_capacity{device="UPS_BMS"} 12.000000953674316
# HELP remain_percent Remain capacity of battery, %
# TYPE remain_percent gauge
remain_percent{device="UPS_BMS"} 100
# HELP up_time Time since last poweron, S
# TYPE up_time counter
up_time{device="UPS_BMS"} 1770773
```

Run prometheus exporter for specified devices (with logging to journald):
```plain
$ ubmsc -e -u http://127.0.0.1:9898/metrics -l ubmsc=debug -j -d UPS_BMS -d SOLAR_BMS
```

Run prometheus pushgateway client continuously to export to VictoriaMetrics:
```plain
$ ubmsc -e -p -u http://127.0.0.1:8428/api/v1/import/prometheus -l ubmsc=info -j -d UPS_BMS -d SOLAR_BMS
```

## Alternative solutions

- [MPP-Solar](https://github.com/jblance/mpp-solar)
  Python module and command-line tools to work with BMS.
  Has support several models of different vendors.
  Doesn't work with JK-BMS with latest firmwares (>11.x).
- [esphome-jk-bms](https://github.com/syssi/esphome-jk-bms)
  Component for ESPHome to interact with JK-BMS.
  Not tested.
