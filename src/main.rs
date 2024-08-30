use clap::Parser;
use config::Config;
use nvml_wrapper::{enum_wrappers::device::TemperatureSensor, error::NvmlError, Device, Nvml};
use std::{process::Command, thread, time::Duration};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: String,
}

#[derive(Debug, Default, serde::Deserialize, PartialEq, Eq)]
struct AppConfig {
    interval: u32,
    tolerance: u32,
    log: bool,
    steps: Vec<Step>,
}

#[derive(Debug, Default, serde::Deserialize, PartialEq, Eq)]
struct Step {
    x: u32,
    y: u32,
}

struct FanSpeedInfo {
    fan_id: u32,
    speed: u32,
}

struct DeviceInfo {
    name: String,
    temperature: u32,
    fan_speeds: Vec<FanSpeedInfo>,
}

impl DeviceInfo {
    pub fn new(device: &Device) -> Result<Self, NvmlError> {
        let mut speeds: Vec<FanSpeedInfo> = Vec::new();
        for i in 0..device.num_fans().unwrap() {
            speeds.push(FanSpeedInfo {
                fan_id: i,
                speed: device.fan_speed(i)?,
            })
        }

        let info = DeviceInfo {
            name: device.name()?,
            temperature: device.temperature(TemperatureSensor::Gpu)?,
            fan_speeds: speeds,
        };

        Ok(info)
    }
}

fn calculate_speed_value(curve: &Vec<Step>, temperature: u32) -> u32 {
    for (idx, _point) in curve.iter().enumerate() {
        if curve[idx].x < temperature && temperature < curve[idx + 1].x {
            let m = (curve[idx + 1].y - curve[idx].y) / (curve[idx + 1].x - curve[idx].x);
            let b = curve[idx].y - (m * curve[idx].x);
            return m * temperature + b;
        }
    }

    return 0;
}

fn set_fan_speed(device_index: u32, target_speed: u32) {
    let _ = Command::new("nvidia-settings")
        .args([
            "-c",
            "0",
            "-a",
            &format!("[gpu:{device_index}]/GPUFanControlState=1"),
            "-a",
            &format!("GPUTargetFanSpeed={target_speed}"),
        ])
        .stdout(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .status();
}

fn daemon(nvml: &Nvml, config: AppConfig) {
    let device = nvml.device_by_index(0).unwrap();
    let mut last_temperature: u32 = 0;

    loop {
        let info = DeviceInfo::new(&device).unwrap();
        let diff = info.temperature.abs_diff(last_temperature);

        if diff >= config.tolerance {
            last_temperature = info.temperature;
            let target_speed = calculate_speed_value(&config.steps, info.temperature);

            if config.log {
                print!(
                    "[LOG]: {}, temperature: {}°C, current speed: ",
                    info.name.as_str(),
                    info.temperature
                );

                for speed in info.fan_speeds {
                    print!("(id: {}, speed: {}% ), ", speed.fan_id, speed.speed);
                }

                print!("target speed: {}%", target_speed);

                println!("");
            }

            set_fan_speed(device.index().unwrap(), target_speed);
        }

        thread::sleep(Duration::from_secs(config.interval.into()))
    }
}

fn main() {
    let args = Args::parse();

    let mut builder = Config::builder();
    builder = builder.add_source(config::File::new(
        args.config.as_str(),
        config::FileFormat::Json,
    ));

    let config: AppConfig = builder.build().unwrap().try_deserialize().unwrap();

    let nvml: Nvml = match Nvml::init() {
        Ok(i) => i,
        Err(_) => panic!("Could not initialize nvml"),
    };

    daemon(&nvml, config);
}
