//! Power management module for dual-battery Christmas ornament.
//!
//! This module handles automatic switching between main and backup batteries
//! using the STM32's Programmable Voltage Detector (PVD) peripheral.
//!
//! # Hardware Design
//!
//! The power system uses two coin cell batteries (BT1 main, BT2 backup) with
//! MIC94050 load switches (Q1, Q2) controlled by active-low signals:
//! - MAIN_POWER_N (PB1): Controls main battery load switch
//! - BACKUP_POWER_N (PA8): Controls backup battery load switch
//!
//! Schottky diodes (D1, D2: BAT54J) OR the outputs together. The switches use
//! make-before-break switching to prevent power loss during transitions.
//!
//! # PVD Operation
//!
//! The PVD monitors VDD and triggers EXTI line 16 when voltage crosses the
//! threshold (2.7V). This wakes the MCU from STOP mode and signals the
//! power monitor task to switch power sources.

use embassy_stm32::{gpio::Output, pac};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use pac::interrupt;

/// EXTI line number for PVD interrupt (fixed at line 16 on STM32)
const PVD_EXTI_LINE: usize = 16;

/// IMR register index for EXTI line 16 (lines 0-31 are in IMR1)
const IMR1_REG_IDX: usize = 0;

/// Power source state for tracking active battery
#[derive(Default)]
pub enum PowerState {
    /// Main battery (BT1) is active
    #[default]
    MainPower,
    /// Backup battery (BT2) is active
    BackupPower,
}

/// Controller for dual-battery power management with automatic failover.
///
/// Controls two active-low load switch enable signals to select between
/// main and backup batteries. Ensures make-before-break switching to
/// prevent power interruption during transitions.
pub struct PowerController {
    /// Active-low enable for main battery load switch (Q1)
    main_power_n: Output<'static>,
    /// Active-low enable for backup battery load switch (Q2)
    backup_power_n: Output<'static>,
    /// Current active power source
    state: PowerState,
}

impl PowerController {
    /// Creates a new PowerController with the specified GPIO outputs.
    ///
    /// # Arguments
    ///
    /// * `main_power_n` - Active-low control for main battery load switch (PB1)
    /// * `backup_power_n` - Active-low control for backup battery load switch (PA8)
    pub fn new(main_power_n: Output<'static>, backup_power_n: Output<'static>) -> Self {
        Self {
            main_power_n,
            backup_power_n,
            state: PowerState::default(),
        }
    }

    /// Initializes the power controller with main battery active.
    ///
    /// Sets main power ON (low) and backup power OFF (high).
    /// Should be called once during system initialization.
    pub fn init_main_power(&mut self) {
        self.main_power_n.set_low();
        self.backup_power_n.set_high();
    }

    /// Handles power source transitions based on voltage detector status.
    ///
    /// Called from the PVD interrupt handler via the power monitor task.
    /// Toggles between main and backup power when voltage drops are detected.
    ///
    /// # Arguments
    ///
    /// * `low_voltage` - True if PVD detected voltage below threshold
    pub fn power_transition(&mut self, low_voltage: bool) {
        if low_voltage {
            self.state = match &self.state {
                PowerState::MainPower => {
                    self.switch_to_backup();
                    PowerState::BackupPower
                }
                PowerState::BackupPower => {
                    self.switch_to_main();
                    PowerState::MainPower
                }
            }
        }
    }

    /// Switches from main to backup battery using make-before-break.
    ///
    /// Enables backup power BEFORE disabling main to prevent power loss.
    fn switch_to_backup(&mut self) {
        self.backup_power_n.set_low(); // Backup ON
        self.main_power_n.set_high(); // Main OFF
    }

    /// Switches from backup to main battery using make-before-break.
    ///
    /// Enables main power BEFORE disabling backup to prevent power loss.
    fn switch_to_main(&mut self) {
        self.main_power_n.set_low(); // Main ON
        self.backup_power_n.set_high(); // Backup OFF
    }
}

/// Static signal for communicating PVD events from interrupt to async task.
///
/// The PVD interrupt handler signals voltage status (true = low voltage)
/// to the power monitor task waiting on this signal.
static PVD_SIGNAL: Signal<CriticalSectionRawMutex, bool> = Signal::new();

/// PVD interrupt handler (EXTI line 16).
///
/// Triggered when VDD crosses the configured threshold (2.7V).
/// Clears the interrupt flag and signals the power monitor task
/// with the current voltage status.
///
/// # Safety
///
/// This interrupt handler accesses PAC registers directly.
#[interrupt]
fn PVD() {
    let exti = pac::EXTI;
    let pwr = pac::PWR;

    // Clear pending interrupt on EXTI line 16
    exti.pr(IMR1_REG_IDX)
        .modify(|w| w.set_line(PVD_EXTI_LINE, true));

    // Check if voltage is below or above the threshold
    let voltage_low = pwr.csr().read().pvdo();

    PVD_SIGNAL.signal(voltage_low);
}

/// Configures the Programmable Voltage Detector (PVD) and EXTI interrupt.
///
/// Sets up PVD to monitor VDD at 2.7V threshold and trigger EXTI line 16
/// on both rising and falling edges. This allows the system to detect
/// battery voltage drops and wake from STOP mode.
///
/// # Configuration
///
/// - PVD threshold: 2.7V (adjustable via `Pls` enum)
/// - EXTI line 16: Rising and falling edge triggers
/// - NVIC: PVD interrupt unmasked
///
/// # Safety
///
/// Directly accesses PAC registers and unmasks NVIC interrupt.
pub fn setup_pvd() {
    // The PAC must be used to configure the PVD
    let pwr = pac::PWR;
    let exti = pac::EXTI;

    // Enable the PWR clock
    pac::RCC.apb1enr().modify(|w| w.set_pwren(true));

    // Configure PVD level (2.7V threshold)
    pwr.cr().modify(|w| w.set_pls(pac::pwr::vals::Pls::V2_7));
    pwr.cr().modify(|w| w.set_pvde(true));

    // Enable EXTI line 16 for PVD
    exti.imr(IMR1_REG_IDX)
        .modify(|w| w.set_line(PVD_EXTI_LINE, true));
    exti.rtsr(IMR1_REG_IDX)
        .modify(|w| w.set_line(PVD_EXTI_LINE, true));
    exti.ftsr(IMR1_REG_IDX)
        .modify(|w| w.set_line(PVD_EXTI_LINE, true));

    // Enable PVD interrupt in the NVIC
    unsafe {
        cortex_m::peripheral::NVIC::unmask(embassy_stm32::interrupt::PVD);
    };
}

/// Async task for monitoring power and handling battery switching.
///
/// Waits for PVD interrupt signals and transitions between main and
/// backup batteries based on voltage detector status. Runs continuously
/// in the background.
///
/// # Arguments
///
/// * `pwr_ctrl` - PowerController instance (takes ownership)
///
/// # Example
///
/// ```no_run
/// spawner.spawn(power_monitor_task(peripherals.pwr_ctrl)).unwrap();
/// ```
#[embassy_executor::task]
pub async fn power_monitor_task(mut pwr_ctrl: PowerController) {
    loop {
        let voltage_low = PVD_SIGNAL.wait().await;
        pwr_ctrl.power_transition(voltage_low);
    }
}
