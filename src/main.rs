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
    ft.set_baud_rate(115200)?;
    ft.set_data_characteristics(BitsPerWord::Bits8, StopBits::Bits1, Parity::No)?;
    let info = ft.device_info()?;
    //println!("Device information: {:?}", info);

    ft.purge_all()?;

    const BUF_SIZE: usize = 100;
    let mut buf: [u8; BUF_SIZE] = [0; BUF_SIZE];
    loop {
        let _: usize = ft.read(&mut buf)?;
        //println!("{}", bytes_read);

        for x in buf.chunks(4) {
            let ch1 = ((((x[0] as u32) << 8) + x[1] as u32) as f32) * 12.2e-3; //mV; 200mV range
            let ch2 = ((((x[2] as u32) << 8) + x[3] as u32) as f32) * 12.2e-3; //mV; 200mV range

            println!("{},{}", ch1, ch2);
            wtr.write_record(&[ch1.to_string(), ch2.to_string()]);
        }
        wtr.flush();
        let keys: Vec<Keycode> = device_state.get_keys();
        if keys.contains(&Keycode::Q) {
            break;
        }
    }
    wtr.flush();
    ft.close()?;
    Ok(())
}
