//! Midi message converter
//!
#![no_std]
#![no_main]

pub mod pedalboard;

use adafruit_feather_rp2040::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_midi::{MidiIn, MidiOut};
use fugit::HertzU32;
use nb::block;
use panic_probe as _;
use pedalboard::rc500::RC500;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use adafruit_feather_rp2040 as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::{
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        pac,
        sio::Sio,
        uart::{DataBits, StopBits, UartConfig, UartPeripheral},
        watchdog::Watchdog,
    },
    Pins, XOSC_CRYSTAL_FREQ,
};

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let uart_pins = (
        pins.tx.into_mode::<bsp::hal::gpio::FunctionUart>(),
        pins.rx.into_mode::<bsp::hal::gpio::FunctionUart>(),
    );

    // set the MIDI baud rate
    let conf = UartConfig::new(
        HertzU32::from_raw(31250),
        DataBits::Eight,
        None,
        StopBits::One,
    );
    let uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(conf, clocks.peripheral_clock.freq())
        .unwrap();

    // Configure Midi
    let (rx, tx) = uart.split();

    let mut midi_in = MidiIn::new(rx);
    let mut midi_out = MidiOut::new(tx);

    let mut rc = RC500::new();
    loop {
        if let Ok(event) = block!(midi_in.read()) {
            info!("received {}", event);

            let messages = pedalboard::handle(event, &mut rc);
            for m in messages.into_iter() {
                info!("send {}", m);
                midi_out.write(&m).ok();
            }
        }
    }
}
