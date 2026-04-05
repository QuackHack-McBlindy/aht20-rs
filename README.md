# **aht20**
 
[![Sponsors](https://img.shields.io/github/sponsors/QuackHack-McBlindy?logo=githubsponsors&label=Sponsor&style=flat&labelColor=ff1493&logoColor=fff&color=rgba(234,74,170,0.5) "")](https://github.com/sponsors/QuackHack-McBlindy) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Sponsor?style=flat&logo=buymeacoffee&logoColor=fff&labelColor=ff1493&color=ff1493)](https://buymeacoffee.com/quackhackmcblindy)




## **Installation**

  
Add **aht20** as a dependency in `Cargo.toml`.

```toml
[dependencies]
aht20 = "0.1.0"
```
  


<br>

## **Example usage**

```rust
#![no_std]
#![no_main]

use aht20::task::{spawn_sensor_task, Reading};
use embassy_executor::Spawner;
use embassy_sync::channel::Channel;
use esp_hal::i2c::master::I2c;
use esp_hal::Blocking;
use critical_section::Mutex;
use core::cell::RefCell;

static SENSOR_CHANNEL: Channel<Reading, 5> = Channel::new();
static I2C_MUTEX: Mutex<RefCell<I2c<'static, Blocking>>> = Mutex::new(RefCell::new(unsafe { core::mem::zeroed() }));

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // ... initialise I2C and fill I2C_MUTEX with the real peripheral ...
    spawn_sensor_task(&spawner, &I2C_MUTEX, &SENSOR_CHANNEL).unwrap();

    // In another task, read from the channel:
    loop {
        let reading = SENSOR_CHANNEL.receive().await;
        defmt::info!("Temp: {}°C, Hum: {}%", reading.temperature_celsius, reading.relative_humidity_percent);
    }
}
``` 

  
# **Features**

- `task` – enables the embassy task and channel integration.

- `defmt` – adds defmt::Format for the Reading type and error logging


<br>

## **Lisence**

**MIT**  
<br>
Contributions are welcomed.


<a href="https://www.buymeacoffee.com/quackhackmcblindy" target="_blank"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" style="height: 60px !important;width: 217px !important;" ></a>

