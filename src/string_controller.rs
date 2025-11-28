//! LED string controller module for Christmas ornament.
//!
//! Controls two independent LED strings (red and green) using D flip-flops.
//! Each flip-flop (SN74LVC1G74) drives a string of 3 LEDs through its Q output.
//!
//! # Hardware Design
//!
//! - U2 (SN74LVC1G74): Controls LSTR1 (3x RED LEDs: LED1, LED3, LED5)
//! - U3 (SN74LVC1G74): Controls LSTR2 (3x GREEN LEDs: LED6, LED2, LED4)
//!
//! Each flip-flop has:
//! - D (data input): Sets the value to be clocked in
//! - CLK (clock): Rising edge latches D input to Q output
//! - PRE_N (preset, active-low): Asynchronously sets Q high
//! - CLR_N (clear, active-low): Asynchronously sets Q low
//! - Q (output): Drives LED string (high = LEDs ON)
//!
//! LEDs are driven with 4.7kΩ resistors for ~255µA current at 3.0V.

use embassy_stm32::gpio::{Input, Output};

/// D flip-flop controller for LED string.
///
/// Wraps GPIO control for a SN74LVC1G74 D flip-flop chip.
/// Provides high-level methods for setting the Q output state.
pub struct FlipFlop {
    /// Active-low preset input (forces Q high when low)
    fpre_n: Output<'static>,
    /// Active-low clear input (forces Q low when low)
    fclr_n: Output<'static>,
    /// Data input (value to be clocked into Q)
    fdata: Output<'static>,
    /// Clock input (rising edge latches D to Q)
    fclk: Output<'static>,
}

impl FlipFlop {
    /// Creates a new FlipFlop controller.
    ///
    /// # Arguments
    ///
    /// * `fpre_n` - Active-low preset GPIO
    /// * `fclr_n` - Active-low clear GPIO
    /// * `fdata` - Data input GPIO
    /// * `fclk` - Clock input GPIO
    pub fn new(
        fpre_n: Output<'static>,
        fclr_n: Output<'static>,
        fdata: Output<'static>,
        fclk: Output<'static>,
    ) -> Self {
        Self {
            fpre_n,
            fclr_n,
            fdata,
            fclk,
        }
    }

    /// Releases asynchronous reset/preset controls.
    ///
    /// Sets both CLR_N and PRE_N low to activate them.
    /// This puts the flip-flop in a known state before normal operation.
    ///
    /// Note: With both active, the actual state depends on chip implementation.
    /// Typically followed by setting data and clocking normally.
    pub fn release_reset(&mut self) {
        self.fclr_n.set_low();
        self.fpre_n.set_low();
    }

    /// Clocks the flip-flop to set Q output high.
    ///
    /// Sets D input high, then pulses CLK to latch the value.
    /// This turns the LED string ON.
    pub fn clock_q_high(&mut self) {
        self.fdata.set_high();
        self.fclk.set_high();
        self.fclk.set_low();
    }

    /// Clocks the flip-flop to set Q output low.
    ///
    /// Sets D input low, then pulses CLK to latch the value.
    /// This turns the LED string OFF.
    pub fn clock_q_low(&mut self) {
        self.fdata.set_low();
        self.fclk.set_high();
        self.fclk.set_low();
    }
}

/// State machine for alternating LED string activation.
///
/// Cycles through four states to create an alternating red/green pattern:
/// GreenOff → Red → RedOff → Green → (repeat)
#[derive(Default)]
enum ActiveString {
    /// Green string is off, transitioning to red
    #[default]
    GreenOff,
    /// Red string is active
    Red,
    /// Red string is off, transitioning to green
    RedOff,
    /// Green string is active
    Green,
}

/// Controller for managing two LED strings with alternating pattern.
///
/// Coordinates two flip-flops to create an alternating red/green display
/// pattern. Each string can also be sensed via feedback inputs (unused
/// in current implementation).
pub struct StringController {
    /// Flip-flop controlling red LED string (LSTR1)
    red_flop: FlipFlop,
    /// Flip-flop controlling green LED string (LSTR2)
    green_flop: FlipFlop,
    /// Feedback input from red string (PB4, currently unused)
    _red_string: Input<'static>,
    /// Feedback input from green string (PA6, currently unused)
    _green_string: Input<'static>,
    /// Current state in the alternating pattern
    active_string: ActiveString,
}

impl StringController {
    /// Creates a new StringController.
    ///
    /// # Arguments
    ///
    /// * `red_flop` - FlipFlop controlling red LED string
    /// * `green_flop` - FlipFlop controlling green LED string
    /// * `red_string` - Feedback input from red string (reserved for future use)
    /// * `green_string` - Feedback input from green string (reserved for future use)
    pub fn new(
        red_flop: FlipFlop,
        green_flop: FlipFlop,
        red_string: Input<'static>,
        green_string: Input<'static>,
    ) -> Self {
        Self {
            red_flop,
            green_flop,
            _red_string: red_string,
            _green_string: green_string,
            active_string: ActiveString::default(),
        }
    }

    /// Resets both LED strings to initial state.
    ///
    /// Releases reset on both flip-flops and returns state machine to
    /// default (GreenOff). Should be called during initialization.
    pub fn reset(&mut self) {
        self.red_flop.release_reset();
        self.green_flop.release_reset();
        self.active_string = ActiveString::default();
    }

    /// Advances to the next state in the LED pattern.
    ///
    /// Implements the state machine:
    /// ```text
    /// GreenOff → Red (turn red ON)
    /// Red      → RedOff (turn red OFF)
    /// RedOff   → Green (turn green ON)
    /// Green    → GreenOff (turn green OFF)
    /// ```
    ///
    /// Call this periodically (e.g., every 3 seconds) to create the
    /// alternating red/green display pattern.
    pub fn activate_next_string(&mut self) {
        self.active_string = match self.active_string {
            ActiveString::GreenOff => {
                self.red_flop.clock_q_high();
                ActiveString::Red
            }
            ActiveString::Red => {
                self.red_flop.clock_q_low();
                ActiveString::RedOff
            }
            ActiveString::RedOff => {
                self.green_flop.clock_q_high();
                ActiveString::Green
            }
            ActiveString::Green => {
                self.green_flop.clock_q_low();
                ActiveString::GreenOff
            }
        }
    }
}
