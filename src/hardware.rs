//! Hardware abstraction and peripheral initialization.
//!
//! This module defines the pin mappings and peripheral initialization
//! for the Christmas ornament hardware.
//!
//! # Pin Assignments
//!
//! ## Power Control
//! - **PB1**: MAIN_POWER_N - Active-low enable for main battery (Q1)
//! - **PA8**: BACKUP_POWER_N - Active-low enable for backup battery (Q2)
//!
//! ## Red LED String Control (LSTR1 via U2)
//! - **PB3**: FCLK1 - Clock input for red flip-flop
//! - **PA15**: FDATA1 - Data input for red flip-flop
//! - **PB5**: FCLR1_N - Active-low clear for red flip-flop
//! - **PB6**: FPRE1_N - Active-low preset for red flip-flop
//! - **PB4**: LSTR1 - Feedback from red LED string (unused)
//!
//! ## Green LED String Control (LSTR2 via U3)
//! - **PB0**: FCLK2 - Clock input for green flip-flop
//! - **PA7**: FDATA2 - Data input for green flip-flop
//! - **PA5**: FCLR2_N - Active-low clear for green flip-flop
//! - **PA4**: FPRE2_N - Active-low preset for green flip-flop
//! - **PA6**: LSTR2 - Feedback from green LED string (unused)
//!
//! ## Low Power & RTC
//! - **PC14**: OSC32_IN - 32.768 kHz crystal input
//! - **PC15**: OSC32_OUT - 32.768 kHz crystal output
//!
//! ## Debug (SWD)
//! - **PA13**: SWDIO
//! - **PA14**: SWCLK

use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};

use crate::power::PowerController;
use crate::string_controller::{FlipFlop, StringController};

/// Top-level peripheral container for the Christmas ornament.
///
/// Owns all hardware controllers and provides initialization
/// from STM32 peripheral singleton.
pub struct Peripherals {
    /// Power management controller (dual-battery system)
    pub pwr_ctrl: PowerController,
    /// LED string pattern controller
    pub str_ctrl: StringController,
}

impl Peripherals {
    /// Initializes all peripherals from STM32 peripheral singleton.
    ///
    /// Consumes the embassy-stm32 Peripherals struct and creates
    /// GPIO outputs/inputs for all hardware controllers.
    ///
    /// # Initial GPIO States
    ///
    /// Power control:
    /// - PB1 (MAIN_POWER_N): Low (main battery ON)
    /// - PA8 (BACKUP_POWER_N): High (backup battery OFF)
    ///
    /// Red flip-flop (U2):
    /// - PB6 (FPRE1_N): Low
    /// - PB5 (FCLR1_N): High
    /// - PA15 (FDATA1): Low
    /// - PB3 (FCLK1): Low
    ///
    /// Green flip-flop (U3):
    /// - PA4 (FPRE2_N): Low
    /// - PA5 (FCLR2_N): High
    /// - PA7 (FDATA2): Low
    /// - PB0 (FCLK2): Low
    ///
    /// # Arguments
    ///
    /// * `p` - STM32 peripheral singleton from embassy_stm32::init()
    ///
    /// # Returns
    ///
    /// Initialized Peripherals struct ready for use
    pub fn new(p: embassy_stm32::Peripherals) -> Self {
        Self {
            pwr_ctrl: PowerController::new(
                Output::new(p.PB1, Level::Low, Speed::Low),
                Output::new(p.PA8, Level::High, Speed::Low),
            ),
            str_ctrl: StringController::new(
                FlipFlop::new(
                    Output::new(p.PB6, Level::Low, Speed::Low),
                    Output::new(p.PB5, Level::High, Speed::Low),
                    Output::new(p.PA15, Level::Low, Speed::Low),
                    Output::new(p.PB3, Level::Low, Speed::Low),
                ),
                FlipFlop::new(
                    Output::new(p.PA4, Level::Low, Speed::Low),
                    Output::new(p.PA5, Level::High, Speed::Low),
                    Output::new(p.PA7, Level::Low, Speed::Low),
                    Output::new(p.PB0, Level::Low, Speed::Low),
                ),
                Input::new(p.PB4, Pull::None),
                Input::new(p.PA6, Pull::None),
            ),
        }
    }
}
