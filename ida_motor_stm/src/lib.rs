#![no_std]

use embedded_hal::pwm::{SetDutyCycle};

pub struct MotorData<PWM>
{
    pwm_pin: PWM,
    max_duty: u16,
}

impl<PWM, E> MotorData<PWM>
where
    PWM: SetDutyCycle<Error= E>,
{
    pub fn new(pwm_pin: PWM, max_duty: u16) -> Self
    {
        Self {pwm_pin, max_duty}
    }
    pub fn pulse_us_ayarla(&mut self, pulse_us: u32) -> Result<(), E>
    {
        let safe_pulse = pulse_us.clamp(1000, 2000);
        let duty = ((safe_pulse * self.max_duty as u32) / 20_000) as u16;
        self.pwm_pin.set_duty_cycle(duty)
    }
    pub fn set_speed(&mut self, binde_bir: u16) -> Result<(), E>
    {
        let binde_bir = binde_bir.clamp(0, 1000);
        let aktarilan_pwm = 1000 + binde_bir as u32;
        self.pulse_us_ayarla(aktarilan_pwm)
    }
    pub fn esc_baslat(&mut self) -> Result<(), E>
    {
        self.pulse_us_ayarla(1000)
    }
}