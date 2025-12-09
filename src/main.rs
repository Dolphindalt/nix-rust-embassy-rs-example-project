//! Firmware for battery-powered Christmas ornament with dual-battery backup.
//!
//! # Overview
//!
//! This firmware controls a low-power Christmas ornament featuring:
//! - Two LED strings (red and green) alternating every 3 seconds
//! - Dual coin cell batteries with automatic failover
//! - Ultra-low power operation using STM32L031G6 in STOP mode
//! - Programmable Voltage Detector (PVD) for battery monitoring
//!
//! # Hardware
//!
//! - **MCU**: STM32L031G6U6 (Cortex-M0+, ultra-low-power)
//! - **Batteries**: 2x coin cells (main + backup)
//! - **Load switches**: 2x MIC94050 for battery selection
//! - **LED drivers**: 2x SN74LVC1G74 D flip-flops
//! - **LEDs**: 6 total (3 red, 3 green) at ~255µA each
//! - **RTC**: 32.768 kHz crystal for timekeeping in STOP mode
//!
//! # Power Management
//!
//! The system starts on the main battery. When PVD detects voltage below
//! 2.7V, it triggers an interrupt that wakes the MCU from STOP mode and
//! switches to the backup battery. This process continues to alternate
//! between batteries as they deplete.
//!
//! # Low Power Operation
//!
//! - MSI oscillator at 66 kHz for minimal active current
//! - Embassy executor automatically enters STOP mode when idle
//! - RTC timer wakes MCU every 3 seconds to update LED pattern
//! - PVD interrupt wakes MCU when battery voltage changes
//!
//! # Module Organization
//!
//! - [`power`] - Dual-battery management and PVD monitoring
//! - [`string_controller`] - LED flip-flop control and pattern state machine
//! - [`hardware`] - Pin mappings and peripheral initialization

#![no_std]
#![no_main]

mod hardware;
mod power;
mod string_controller;

use embassy_executor::Spawner;
use embassy_stm32::{
    Config,
    rcc::{LsConfig, LseConfig, mux::ClockMux},
    time::Hertz,
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

use hardware::Peripherals;
use power::{power_monitor_task, setup_pvd};

/// LED pattern cycle time in seconds.
///
/// The LED strings alternate between red and green at this interval.
/// Each string is ON for this duration, then OFF for this duration.
const LED_CYCLE_SECS: u64 = 3;

/// Creates a low-power clock configuration for STM32L031.
///
/// # Clock Settings
///
/// - **MSI**: 66 kHz in normal mode, 2.097 MHz in debug mode (for reliable debugging)
/// - **System clock**: MSI (no PLL)
/// - **LSE**: 32.768 kHz external crystal for RTC
/// - **Voltage scale**: Range 1 (1.8V core for low power)
///
/// This configuration prioritizes power efficiency over performance,
/// suitable for battery-powered applications with infrequent wake-ups.
/// In debug mode, uses higher clock speed to keep debug connection alive.
///
/// # Returns
///
/// Configured RCC settings for embassy-stm32 initialization
fn create_low_power_config() -> embassy_stm32::rcc::Config {
    embassy_stm32::rcc::Config {
        #[cfg(feature = "debug-mode")]
        msi: Some(embassy_stm32::rcc::MSIRange::RANGE2M),
        #[cfg(not(feature = "debug-mode"))]
        msi: Some(embassy_stm32::rcc::MSIRange::RANGE66K),
        hsi: false,
        hse: None,
        pll: None,
        sys: embassy_stm32::rcc::Sysclk::MSI,
        ahb_pre: embassy_stm32::rcc::AHBPrescaler::DIV1,
        apb1_pre: embassy_stm32::rcc::APBPrescaler::DIV1,
        apb2_pre: embassy_stm32::rcc::APBPrescaler::DIV1,
        ls: LsConfig {
            rtc: embassy_stm32::rcc::RtcClockSource::LSE,
            lsi: false,
            lse: Some(LseConfig {
                frequency: Hertz::hz(32768),
                mode: embassy_stm32::rcc::LseMode::Oscillator(embassy_stm32::rcc::LseDrive::Low),
            }),
        },
        voltage_scale: embassy_stm32::rcc::VoltageScale::RANGE1,
        mux: ClockMux::default(),
    }
}

/// Main entry point for Christmas ornament firmware.
///
/// # Initialization Sequence
///
/// 1. Configure clocks for low power operation (66 kHz MSI)
/// 2. Initialize STM32 peripherals
/// 3. Setup PVD for battery voltage monitoring
/// 4. Initialize GPIO and controllers
/// 5. Activate main battery and reset LED state
/// 6. Spawn background task for power monitoring
/// 7. Enter main loop alternating LED pattern
///
/// # Main Loop
///
/// The main loop cycles through LED states every 3 seconds:
/// - GreenOff → Red ON → Red OFF → Green ON → (repeat)
///
/// Between state changes, the MCU enters STOP mode automatically,
/// waking only when the RTC timer expires or PVD triggers.
///
/// # Spawned Tasks
///
/// - **power_monitor_task**: Handles battery switching on PVD events
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    config.rcc = create_low_power_config();

    let p = embassy_stm32::init(config);

    #[cfg(feature = "debug-mode")]
    defmt::info!("Christmas ornament firmware starting...");

    // Wait 3 seconds after boot to allow debugger connection
    // before entering STOP mode. Negligible battery impact.
    #[cfg(feature = "debug-mode")]
    defmt::info!("Waiting 3 seconds for debugger connection...");

    Timer::after_secs(3).await;

    #[cfg(feature = "debug-mode")]
    defmt::info!("Setting up PVD...");

    setup_pvd();

    #[cfg(feature = "debug-mode")]
    defmt::info!("Initializing peripherals...");

    let mut peripherals = Peripherals::new(p);

    #[cfg(feature = "debug-mode")]
    defmt::info!("Initializing power controller...");

    peripherals.pwr_ctrl.init_main_power();

    #[cfg(feature = "debug-mode")]
    defmt::info!("Resetting LED controllers...");

    peripherals.str_ctrl.reset();

    #[cfg(feature = "debug-mode")]
    defmt::info!("Spawning power monitor task...");

    spawner
        .spawn(power_monitor_task(peripherals.pwr_ctrl))
        .unwrap();

    #[cfg(feature = "debug-mode")]
    defmt::info!("Entering main LED cycle loop...");

    loop {
        #[cfg(feature = "debug-mode")]
        defmt::info!("Activating next string...");

        peripherals.str_ctrl.activate_next_string();

        #[cfg(feature = "debug-mode")]
        defmt::info!("Sleeping for {} seconds", LED_CYCLE_SECS);

        Timer::after_secs(LED_CYCLE_SECS).await;
    }
}
