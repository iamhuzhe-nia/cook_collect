use chrono;
use corncobs::*;
use device_query::{DeviceQuery, DeviceState, Keycode};
use libftd2xx::{BitsPerWord, FtStatus, Ftdi, FtdiCommon, Parity, StopBits};

fn main() -> Result<(), FtStatus> {
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

    let mut serial_buffer_index = 0usize;
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
                        serial_buffer_index = 0;
                        serial_buffer[0] = header[0];
                        state = Fsm::COBS;
                        //println!("jmp to COBS with {serial_buffer_index}");
                    }
                }
            }
            Fsm::COBS => {
                if ft.queue_status()? > 0 {
                    let _: usize = ft.read(&mut header)?;
                    serial_buffer_index += 1;
                    serial_buffer[serial_buffer_index] = header[0];
                    //println!("{serial_buffer_index}, {}", header[0]);
                    if header[0] == 0u8 {
                        state = Fsm::IDLE;
                        if serial_buffer_index == 133 {
                            let decoded_data_length = decode_buf(
                                &serial_buffer[..=serial_buffer_index],
                                &mut decoded_data,
                            )
                            .unwrap();
                        //    println!("decoded bytes = {decoded_data_length}");
                            if decoded_data_length == BUF_SIZE {
                                //got the right package size
                                let mut adc_data = Vec::new();
                                for i in (0..BUF_SIZE).step_by(2) {
                                    adc_data.push(
                                        (((decoded_data[i] as u16) << 8)
                                            + decoded_data[i + 1] as u16)
                                            .to_string(),
                                    ); // send as big-endian
                                }
                                println!("{}", adc_data[1]);
                                wtr.write_record(&adc_data).unwrap();
                                wtr.flush().unwrap();
                            }else{
                                println!("cobs error: decoded length={}", decoded_data_length);
                            }
                        } else {
                            println!("error: ser index  = {serial_buffer_index}");
                            // break;
                        }
                    }
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
