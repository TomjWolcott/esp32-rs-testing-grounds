use std::time::Duration;
use esp_idf_svc::hal::gpio::*;
use log::info;

#[derive(Debug)]
pub enum BldcPhase {
    AB,
    AC,
    BC,
    BA,
    CA,
    CB
}

pub struct BldcDriver<'a, P1: Pin, P2: Pin, P3: Pin, P4: Pin, P5: Pin, P6: Pin> {
    a_gnd: PinDriver<'a, P1, Output>,
    a_pow: PinDriver<'a, P2, Output>,
    b_gnd: PinDriver<'a, P3, Output>,
    b_pow: PinDriver<'a, P4, Output>,
    c_gnd: PinDriver<'a, P5, Output>,
    c_pow: PinDriver<'a, P6, Output>
}

impl<'a,
    P1: Pin + OutputPin, P2: Pin + OutputPin,
    P3: Pin + OutputPin, P4: Pin + OutputPin,
    P5: Pin + OutputPin, P6: Pin + OutputPin
> BldcDriver<'a, P1, P2, P3, P4, P5, P6> {
    pub fn new(
        (a_gnd, a_pow): (P1, P2),
        (b_gnd, b_pow): (P3, P4),
        (c_gnd, c_pow): (P5, P6)
    ) -> anyhow::Result<Self> {
        let mut driver = BldcDriver {
            a_gnd: PinDriver::output(a_gnd)?,
            a_pow: PinDriver::output(a_pow)?,
            b_gnd: PinDriver::output(b_gnd)?,
            b_pow: PinDriver::output(b_pow)?,
            c_gnd: PinDriver::output(c_gnd)?,
            c_pow: PinDriver::output(c_pow)?
        };

        driver.init()?;

        Ok(driver)
    }

    pub fn init(&mut self) -> anyhow::Result<()> {
        info!("Init-ing driver!");
        self.a_gnd.set_low()?;
        self.a_pow.set_low()?;
        self.b_gnd.set_low()?;
        self.b_pow.set_low()?;
        self.c_gnd.set_low()?;
        self.c_pow.set_low()?;
        Ok(())
    }

    pub fn send_phase(&mut self, phase_duration: Duration, phase: &BldcPhase) -> anyhow::Result<()> {
        match phase {
            BldcPhase::AB => {
                self.a_pow.set_high()?;
                self.b_gnd.set_high()?;
            }
            BldcPhase::AC => {
                self.a_pow.set_high()?;
                self.c_gnd.set_high()?;
            }
            BldcPhase::BC => {
                self.b_pow.set_high()?;
                self.c_gnd.set_high()?;
            }
            BldcPhase::BA => {
                self.b_pow.set_high()?;
                self.a_gnd.set_high()?;
            }
            BldcPhase::CA => {
                self.c_pow.set_high()?;
                self.a_gnd.set_high()?;
            }
            BldcPhase::CB => {
                self.c_pow.set_high()?;
                self.b_gnd.set_high()?;
            }
        }

        info!("Phase: {:?}", phase);

        spin_sleep::sleep(phase_duration);

        match phase {
            BldcPhase::AB => {
                self.a_pow.set_low()?;
                self.b_gnd.set_low()?;
            }
            BldcPhase::AC => {
                self.a_pow.set_low()?;
                self.c_gnd.set_low()?;
            }
            BldcPhase::BC => {
                self.b_pow.set_low()?;
                self.c_gnd.set_low()?;
            }
            BldcPhase::BA => {
                self.b_pow.set_low()?;
                self.a_gnd.set_low()?;
            }
            BldcPhase::CA => {
                self.c_pow.set_low()?;
                self.a_gnd.set_low()?;
            }
            BldcPhase::CB => {
                self.c_pow.set_low()?;
                self.b_gnd.set_low()?;
            }
        }
        info!("Phase off: {:?}", phase);

        Ok(())
    }

    pub fn send_sequence(&mut self, pulse_width: Duration, interval_width: Duration) -> anyhow::Result<()> {
        const PATTERN: [BldcPhase; 6] = [
            BldcPhase::AB,
            BldcPhase::AC,
            BldcPhase::BC,
            BldcPhase::BA,
            BldcPhase::CA,
            BldcPhase::CB
        ];

        let between_pulse_duration = interval_width / PATTERN.len() as u32 - pulse_width;

        for phase in PATTERN.iter() {
            self.send_phase(pulse_width, phase)?;
            spin_sleep::sleep(between_pulse_duration);
        }

        Ok(())
    }
}