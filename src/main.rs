use chrono;
use corncobs::*;
use device_query::{DeviceQuery, DeviceState, Keycode};
use libftd2xx::{BitsPerWord, FtStatus, Ftdi, FtdiCommon, Parity, StopBits};


use argh::FromArgs;

#[derive(FromArgs)]
/// set time base for collected data
struct TimeBase {
    /// time step in seconds
    #[argh(option, short = 't', default = "0.1")]
    t_sec: f64, // 0.1 sec default
}


fn main() -> Result<(), FtStatus> {
    let time_base: TimeBase = argh::from_env();

    let device_state = DeviceState::new();
    let mut dt = chrono::offset::Local::now()
        .format("%Y-%m-%d-%H-%M-%S")
        .to_string();
    dt.push_str(".csv");
    let mut wtr = csv::Writer::from_path(dt).unwrap();

    let mut ft = Ftdi::new()?;
    ft.set_baud_rate(115200)?;
    ft.set_data_characteristics(BitsPerWord::Bits8, StopBits::Bits1, Parity::No)?;
    ft.purge_all()?;

    // incoming COBS from serial
    enum Fsm {
        IDLE,
        RESYNC,
        COBS,
    }
    let mut state = Fsm::RESYNC;

    let mut serial_buffer = [0u8; 400];

    const BUF_SIZE: usize = 4; // 4 + 2
    let mut decoded_data = [0u8; 400];
    let mut header: [u8; 1] = [0; 1];
    let mut x_index: f64 = 0.0;
    loop {
        match state {
            Fsm::RESYNC => {
                if ft.queue_status()? > 0 {
                    let _: usize = ft.read(&mut header)?;
                    if header[0] == 0u8 {
                        state = Fsm::IDLE;
                        // println!("jmp to IDLE");
                    }
                }
            }

            Fsm::IDLE => {
                if ft.queue_status()? > 0 {
                    let _: usize = ft.read(&mut header)?;
                    if header[0] != 0u8 {
                        serial_buffer[0] = header[0];
                        state = Fsm::COBS;
                        //println!("jmp to COBS with {serial_buffer_index}");
                    }
                }
            }
            Fsm::COBS => {
                ft.read(&mut serial_buffer[1..(BUF_SIZE + 2)])?;
                if serial_buffer[BUF_SIZE + 1] == 0 {
                    state = Fsm::IDLE;
                    let decoded_data_length =
                        decode_buf(&serial_buffer[..(BUF_SIZE + 2)], &mut decoded_data).unwrap();

                    if decoded_data_length == BUF_SIZE {
                        //got the right package size
                        let adc_data = (2.0 * 0.8 * (((decoded_data[0] as u32) << 24)
                            + ((decoded_data[1] as u32) << 16)
                            + ((decoded_data[2] as u32) << 8)
                            + (decoded_data[3] as u32)) as f64 / 1000.0)
                            .to_string();

                        x_index += time_base.t_sec;
                        println!("{}, {}", x_index, adc_data);
                        //println!("{}", adc_data[0]);

                        wtr.write_record(&[x_index.to_string(), adc_data]).unwrap();
                        wtr.flush().unwrap();
                    } else {
                        println!("cobs error: decoded length={}", decoded_data_length);
                    }
                } else {
                    //println!("error COBS");
                    state = Fsm::RESYNC;
                }
            }
        }

        let keys: Vec<Keycode> = device_state.get_keys();
        if keys.contains(&Keycode::Q) {
            break;
        }
    }
    wtr.flush().unwrap();
    ft.close()?;
    Ok(())
}
