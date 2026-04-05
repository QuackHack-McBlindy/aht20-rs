#![no_std]

use core::fmt;
use embedded_hal::i2c::I2c as BlockingI2c;
use embedded_hal_async::i2c::I2c as AsyncI2c;

/// AHT20 I2C address.
pub const AHT20_ADDR: u8 = 0x38;

/// Possible errors when communicating with the AHT20.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// I2C bus error (e.g. no acknowledge, clock stretch timeout)
    I2c,
    /// Sensor not ready (calibration bit not set)
    NotReady,
    /// CRC check failed (if CRC is enabled in future)
    CrcMismatch,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::I2c => write!(f, "I2C communication error"),
            Error::NotReady => write!(f, "Sensor not ready"),
            Error::CrcMismatch => write!(f, "CRC mismatch"),
        }
    }
}

/// Blocking (synchronous) read using `embedded_hal::i2c::I2c`.
/// This function blocks the current thread/task for ~80ms.
pub fn read_blocking<I2C: BlockingI2c>(i2c: &mut I2C) -> Result<(f32, f32), Error> {
    // Initialisation command (required after power‑up)
    let init_cmd = [0xBE, 0x08, 0x00];
    i2c.write(AHT20_ADDR, &init_cmd).map_err(|_| Error::I2c)?;
    // Wait for sensor to initialise
    block_for_ms(10);

    // Trigger measurement
    let measure_cmd = [0xAC, 0x33, 0x00];
    i2c.write(AHT20_ADDR, &measure_cmd).map_err(|_| Error::I2c)?;
    block_for_ms(80);

    // Read 6 bytes of data
    let mut buf = [0u8; 6];
    i2c.read(AHT20_ADDR, &mut buf).map_err(|_| Error::I2c)?;

    // Check status byte (bit 7 = busy, bit 3 = calibrated)
    if buf[0] & 0x80 != 0 {
        return Err(Error::NotReady);
    }

    // Parse humidity (20 bits)
    let raw_hum = ((buf[1] as u32) << 12) | ((buf[2] as u32) << 4) | ((buf[3] as u32) >> 4);
    // Parse temperature (20 bits)
    let raw_temp = (((buf[3] as u32) & 0x0F) << 16) | ((buf[4] as u32) << 8) | (buf[5] as u32);

    let humidity = (raw_hum as f32) * 100.0 / (1 << 20) as f32;
    let temperature = (raw_temp as f32) * 200.0 / (1 << 20) as f32 - 50.0;

    Ok((temperature, humidity))
}

/// Async read using `embedded_hal_async::i2c::I2c`.
/// Requires an async delay implementation (e.g. `embassy_time`).
pub async fn read_async<I2C: AsyncI2c, D: embassy_time::Delay>(
    i2c: &mut I2C,
    delay: &mut D,
) -> Result<(f32, f32), Error> {
    let init_cmd = [0xBE, 0x08, 0x00];
    i2c.write(AHT20_ADDR, &init_cmd).await.map_err(|_| Error::I2c)?;
    delay.delay_ms(10).await;

    let measure_cmd = [0xAC, 0x33, 0x00];
    i2c.write(AHT20_ADDR, &measure_cmd).await.map_err(|_| Error::I2c)?;
    delay.delay_ms(80).await;

    let mut buf = [0u8; 6];
    i2c.read(AHT20_ADDR, &mut buf).await.map_err(|_| Error::I2c)?;

    if buf[0] & 0x80 != 0 {
        return Err(Error::NotReady);
    }

    let raw_hum = ((buf[1] as u32) << 12) | ((buf[2] as u32) << 4) | ((buf[3] as u32) >> 4);
    let raw_temp = (((buf[3] as u32) & 0x0F) << 16) | ((buf[4] as u32) << 8) | (buf[5] as u32);

    let humidity = (raw_hum as f32) * 100.0 / (1 << 20) as f32;
    let temperature = (raw_temp as f32) * 200.0 / (1 << 20) as f32 - 50.0;

    Ok((temperature, humidity))
}

/// Blocking delay in milliseconds (simple busy loop, acceptable for short delays).
fn block_for_ms(ms: u64) {
    let loops = ms * 10_000; // rough calibration, adjust as needed
    for _ in 0..loops {
        core::hint::spin_loop();
    }
}

// -----------------------------------------------------------------------------
// Optional embassy task (enabled with the "task" feature)
// -----------------------------------------------------------------------------
#[cfg(feature = "task")]
pub mod task {
    use super::*;
    use embassy_sync::channel::Channel;
    use embassy_time::{Duration, Timer};

    /// A reading from the AHT20 sensor.
    #[derive(Debug, Clone, Copy, defmt::Format)]
    pub struct Reading {
        pub temperature_celsius: f32,
        pub relative_humidity_percent: f32,
    }

    /// Spawns an embassy task that periodically reads the sensor and sends
    /// readings into the provided channel.
    ///
    /// # Example
    /// ```rust,ignore
    /// use aht20::task::{spawn_sensor_task, Reading};
    ///
    /// static SENSOR_CHANNEL: Channel<Reading, 5> = Channel::new();
    ///
    /// spawn_sensor_task(spawner, i2c_mutex, &SENSOR_CHANNEL).unwrap();
    /// ```
    #[embassy_executor::task]
    pub async fn sensor_task<I2C: BlockingI2c>(
        i2c_mutex: &'static critical_section::Mutex<core::cell::RefCell<I2C>>,
        sender: &'static Channel<Reading, 5>,
    ) {
        loop {
            let mut i2c = i2c_mutex.borrow_ref_mut();
            let result = read_blocking(&mut *i2c);
            drop(i2c); // release lock

            match result {
                Ok((temp, hum)) => {
                    let reading = Reading {
                        temperature_celsius: temp,
                        relative_humidity_percent: hum,
                    };
                    sender.send(reading).await;
                }
                Err(e) => {
                    #[cfg(feature = "defmt")]
                    defmt::error!("AHT20 error: {}", defmt::Debug2Format(&e));
                }
            }
            Timer::after(Duration::from_secs(60)).await;
        }
    }

    /// Convenience function to spawn the sensor task.
    /// Returns `Ok(())` on success, or `Err` if the spawner fails.
    pub fn spawn_sensor_task<I2C: BlockingI2c>(
        spawner: &embassy_executor::Spawner,
        i2c_mutex: &'static critical_section::Mutex<core::cell::RefCell<I2C>>,
        sender: &'static Channel<Reading, 5>,
    ) -> Result<(), embassy_executor::SpawnError> {
        spawner.spawn(sensor_task(i2c_mutex, sender))
    }
}
