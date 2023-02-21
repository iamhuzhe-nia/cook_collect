use chrono;
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
    //let info = ft.device_info()?;
    //println!("Device information: {:?}", info);

    ft.purge_all()?;

    const BUF_SIZE: usize = 34 * 4;
    let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
    let mut header:[u8;1] = [0;1];
    loop {
        let _: usize = ft.read(&mut header)?;
        if header[0] == 0xfe {
            let _: usize = ft.read(&mut buf)?;
            let mut adc_data = Vec::new();
            for i in (0..BUF_SIZE).step_by(2){
                adc_data.push((((buf[i] as u16)<<8) + buf[i+1] as u16).to_string());
            }
            wtr.write_record(&adc_data).unwrap();
            wtr.flush().unwrap();

            // //println!("{}", bytes_read);

            // let ch1 = ((buf[0] as u16) << 8) + buf[1] as u16;
            // let ch2 = ((buf[2] as u16) << 8) + buf[3] as u16;

            // println!("{},{}", ch1, ch2);
            // wtr.write_record(&[ch1.to_string(), ch2.to_string()]).unwrap();

            // wtr.flush().unwrap();











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
