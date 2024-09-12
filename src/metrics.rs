use crate::{CellData, DeviceId, DeviceInfo, Result};
use prometheus::{default_registry, Counter, Gauge, GaugeVec, Opts, Registry};

pub trait Scrapeable {
    fn scrape(&self, _metrics: &Metrics) {}
}

macro_rules! metrics_impl {
    ( $($class:ident {
        $($name:ident: $kind:ident: $type:ident: $help:literal;)*
    })* ) => {
        /// Metrics for Prometheus exporter
        #[derive(Clone)]
        pub struct Metrics {
            $($($name: metrics_impl!(@type $kind),)*)*
        }

        impl Metrics {
            /// Instantiate metrics for cell data of specified device
            pub fn new(device_id: &DeviceId) -> Result<Self> {
                let device_id = device_id.to_string();

                $($(let $name = create::$kind(&device_id, stringify!($name), $help)?;)*)*

                Ok(Self {
                    $($($name,)*)*
                })
            }

            /// Register metrics
            pub fn register(&self, registry: Option<&Registry>) -> Result<()> {
                let registry = registry.unwrap_or(default_registry());
                $($(registry.register(Box::new(self.$name.clone()))?;)*)*
                Ok(())
            }

            /// Update metrics using provided data
            pub fn scrape<T: Scrapeable>(&self, data: &T) {
                data.scrape(self);
            }
        }

        $(impl Scrapeable for $class {
            fn scrape(&self, metrics: &Metrics) {
                $(update::$kind(&metrics.$name, metrics_impl!(@conv $kind, $type, self.$name));)*
            }
        })*
    };

    (@type counter) => { Counter };
    (@type gauge) => { Gauge };
    (@type gauges) => { GaugeVec };

    (@conv counter, f32, $val:expr) => { $val };
    (@conv counter, usize, $val:expr) => { $val as _ };
    (@conv gauge, f32, $val:expr) => { $val };
    (@conv gauge, u8, $val:expr) => { $val as _ };
    (@conv gauges, f32, $val:expr) => { &$val[..] };
}

metrics_impl! {
    DeviceInfo {
        poweron_times: counter: usize: "Number of poweron cicles";
    }
    CellData {
        cell_voltage: gauges: f32: "Voltages of cells, V";
        average_cell_voltage: gauge: f32: "Average voltage of cells, V";
        delta_cell_voltage: gauge: f32: "Delta voltage of cells, V";
        balance_current: gauge: f32: "Cells balance current, A";
        cell_resistance: gauges: f32: "Resistances of cells, Ω";
        battery_voltage: gauge: f32: "Voltage of battery, V";
        battery_power: gauge: f32: "Power of battery, W";
        battery_current: gauge: f32: "Current of battery, A";
        battery_temperature: gauges: f32: "Temperatures of battery, ℃";
        mosfet_temperature: gauge: f32: "Temperature of mosfet, ℃";
        remain_percent: gauge: u8: "Remain capacity of battery, %";
        remain_capacity: gauge: f32: "Remain capacity of battery, A·h";
        cycle_count: counter: usize: "Number of battery cicles";
        cycle_capacity: counter: f32: "Cycle capacity, A·h";
        up_time: counter: usize: "Time since last poweron, S";
    }
}

mod create {
    use super::*;

    const DEVICE_ID_LABEL: &str = "device";
    const CELL_INDEX_LABEL: &str = "cell";

    pub fn counter(device_id: &str, name: &str, help: &str) -> Result<Counter> {
        Ok(Counter::with_opts(
            Opts::new(name, help).const_label(DEVICE_ID_LABEL, device_id),
        )?)
    }

    pub fn gauge(device_id: &str, name: &str, help: &str) -> Result<Gauge> {
        Ok(Gauge::with_opts(
            Opts::new(name, help).const_label(DEVICE_ID_LABEL, device_id),
        )?)
    }

    pub fn gauges(device_id: &str, name: &str, help: &str) -> Result<GaugeVec> {
        Ok(GaugeVec::new(
            Opts::new(name, help).const_label(DEVICE_ID_LABEL, device_id),
            &[CELL_INDEX_LABEL],
        )?)
    }
}

mod update {
    use super::*;

    pub fn counter(counter: &Counter, value: f32) {
        let value = value as f64;
        let old_value = counter.get();
        if value > old_value {
            counter.inc_by(value - old_value);
        } else {
            counter.reset();
            counter.inc_by(value);
        }
    }

    pub fn gauge(gauge: &Gauge, value: f32) {
        gauge.set(value as _);
    }

    pub fn gauges(gauges: &GaugeVec, values: &[f32]) {
        for (index, value) in values.iter().enumerate() {
            gauges.with_label_values(&[idx2str(index)]).set(*value as _);
        }
    }

    fn idx2str(index: usize) -> &'static str {
        match index {
            0 => "0",
            1 => "1",
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            6 => "6",
            7 => "7",
            8 => "8",
            9 => "9",
            10 => "10",
            11 => "11",
            12 => "12",
            13 => "13",
            14 => "14",
            15 => "15",
            16 => "16",
            17 => "17",
            18 => "18",
            19 => "19",
            20 => "20",
            21 => "21",
            22 => "22",
            23 => "23",
            24 => "24",
            25 => "25",
            26 => "26",
            27 => "27",
            28 => "28",
            29 => "29",
            30 => "30",
            31 => "31",
            _ => "N",
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use prometheus::{Encoder, Registry, TextEncoder};

    #[test]
    fn metrics() {
        let registry = Registry::new();
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();

        let metrics = Metrics::new(&DeviceId::Name("UPS_BMS".into())).unwrap();
        metrics.register(Some(&registry)).unwrap();

        let device_info = DeviceInfo {
            device_model: "JK_BD4A8S4P".into(),
            hardware_version: "15A".into(),
            software_version: "15.26".into(),
            up_time: 1707500,
            poweron_times: 1,
            device_name: "UPS_BMS".into(),
            device_passcode: "1234".into(),
            manufacturing_date: "240818".into(),
            serial_number: "40531310629".into(),
            passcode: "000".into(),
            userdata: "JK-BMS".into(),
            setup_passcode: "123456789".into(),
            userdata2: "JK-BMS".into(),
        };

        let cell_data = CellData {
            cell_voltage: vec![2.384, 2.384, 2.383, 2.384, 2.384, 2.384],
            average_cell_voltage: 2.384,
            delta_cell_voltage: 0.001,
            balance_current: 1.024,
            cell_resistance: vec![0.138, 0.137, 0.14, 0.138, 0.139, 0.139],
            battery_voltage: 14.304,
            battery_power: 2.231,
            battery_current: 0.156,
            battery_temperature: vec![23.2, 23.6],
            mosfet_temperature: 25.4,
            remain_percent: 100,
            remain_capacity: 12.0,
            nominal_capacity: 12.0,
            cycle_count: 1,
            cycle_capacity: 18.464,
            up_time: 1707600,
        };

        metrics.scrape(&device_info);
        metrics.scrape(&cell_data);

        encoder.encode(&registry.gather(), &mut buffer).unwrap();
        let text = String::from_utf8(buffer).unwrap();

        println!("{text}");
        assert_eq!(
            text,
            r#"# HELP average_cell_voltage Average voltage of cells, V
# TYPE average_cell_voltage gauge
average_cell_voltage{device="UPS_BMS"} 2.384000062942505
# HELP balance_current Cells balance current, A
# TYPE balance_current gauge
balance_current{device="UPS_BMS"} 1.0240000486373901
# HELP battery_current Current of battery, A
# TYPE battery_current gauge
battery_current{device="UPS_BMS"} 0.15600000321865082
# HELP battery_power Power of battery, W
# TYPE battery_power gauge
battery_power{device="UPS_BMS"} 2.2309999465942383
# HELP battery_temperature Temperatures of battery, ℃
# TYPE battery_temperature gauge
battery_temperature{cell="0",device="UPS_BMS"} 23.200000762939453
battery_temperature{cell="1",device="UPS_BMS"} 23.600000381469727
# HELP battery_voltage Voltage of battery, V
# TYPE battery_voltage gauge
battery_voltage{device="UPS_BMS"} 14.303999900817871
# HELP cell_resistance Resistances of cells, Ω
# TYPE cell_resistance gauge
cell_resistance{cell="0",device="UPS_BMS"} 0.1379999965429306
cell_resistance{cell="1",device="UPS_BMS"} 0.13699999451637268
cell_resistance{cell="2",device="UPS_BMS"} 0.14000000059604645
cell_resistance{cell="3",device="UPS_BMS"} 0.1379999965429306
cell_resistance{cell="4",device="UPS_BMS"} 0.13899999856948853
cell_resistance{cell="5",device="UPS_BMS"} 0.13899999856948853
# HELP cell_voltage Voltages of cells, V
# TYPE cell_voltage gauge
cell_voltage{cell="0",device="UPS_BMS"} 2.384000062942505
cell_voltage{cell="1",device="UPS_BMS"} 2.384000062942505
cell_voltage{cell="2",device="UPS_BMS"} 2.382999897003174
cell_voltage{cell="3",device="UPS_BMS"} 2.384000062942505
cell_voltage{cell="4",device="UPS_BMS"} 2.384000062942505
cell_voltage{cell="5",device="UPS_BMS"} 2.384000062942505
# HELP cycle_capacity Cycle capacity, A·h
# TYPE cycle_capacity counter
cycle_capacity{device="UPS_BMS"} 18.464000701904297
# HELP cycle_count Number of battery cicles
# TYPE cycle_count counter
cycle_count{device="UPS_BMS"} 1
# HELP delta_cell_voltage Delta voltage of cells, V
# TYPE delta_cell_voltage gauge
delta_cell_voltage{device="UPS_BMS"} 0.0010000000474974513
# HELP mosfet_temperature Temperature of mosfet, ℃
# TYPE mosfet_temperature gauge
mosfet_temperature{device="UPS_BMS"} 25.399999618530273
# HELP poweron_times Number of poweron cicles
# TYPE poweron_times counter
poweron_times{device="UPS_BMS"} 1
# HELP remain_capacity Remain capacity of battery, A·h
# TYPE remain_capacity gauge
remain_capacity{device="UPS_BMS"} 12
# HELP remain_percent Remain capacity of battery, %
# TYPE remain_percent gauge
remain_percent{device="UPS_BMS"} 100
# HELP up_time Time since last poweron, S
# TYPE up_time counter
up_time{device="UPS_BMS"} 1707600
"#
        );
        //assert!(false);
    }
}
