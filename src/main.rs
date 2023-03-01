use chrono;
use corncobs::*;
use device_query::{DeviceQuery, DeviceState, Keycode};
use libftd2xx::{BitsPerWord, FtStatus, Ftdi, FtdiCommon, Parity, StopBits};

use argh::FromArgs;

#[derive(FromArgs)]
/// select E1 to E64 to stream out
struct StreamData{
    /// select single electrode data out: 1 to 64
    #[argh(option, short='e', default="1")]
    electrode: usize, // 1 to 64
}

fn main() -> Result<(), FtStatus> {
    let stream: StreamData = argh::from_env();



    let device_state = DeviceState::new();
    let mut dt = chrono::offset::Local::now()
        .format("%Y-%m-%d-%H-%M-%S")
        .to_string();
    dt.push_str(".csv");
    let mut wtr = csv::Writer::from_path(dt).unwrap();

    let mut ft = Ftdi::new()?;
    ft.set_baud_rate(3000000)?;
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

    const BUF_SIZE: usize = 33 * 4;
    let mut decoded_data = [0u8; 400];
    let mut header: [u8; 1] = [0; 1];
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
                ft.read(&mut serial_buffer[1..134])?; // 33 * 4 = 132  + 2-byte extra from COBS
                if serial_buffer[133] == 0 {
                    state = Fsm::IDLE;
                    let decoded_data_length =
                        decode_buf(&serial_buffer[..134], &mut decoded_data).unwrap();
                    //    println!("decoded bytes = {decoded_data_length}");
                    if decoded_data_length == BUF_SIZE {
                        //got the right package size
                        let mut adc_data = Vec::new();
                        for i in (0..BUF_SIZE).step_by(2) {
                            adc_data.push(
                                (((decoded_data[i] as u16) << 8) + decoded_data[i + 1] as u16)
                                    .to_string(),
                            ); // receive as big-endian
                        }
                        println!("{}", adc_data[stream.electrode-1]);
                        wtr.write_record(&adc_data).unwrap();
                        wtr.flush().unwrap();
                    } else {
                        println!("cobs error: decoded length={}", decoded_data_length);
                    }
                } else {
                    println!("error COBS");
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
