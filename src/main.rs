mod error_views;
mod hex_conversion;
mod util;

use hex_conversion::U8_TO_HEX;

use std::collections::HashMap;
use std::env;
use std::io;

use cursive::event::Event;
use cursive::traits::*;
use cursive::views::{Dialog, EditView, HexArea, LinearLayout, TextView};
use cursive::Cursive;

const HEX_AREA_ID: &'static str = "content";
const GOTO_ADDRESS_ID: &'static str = "address";

fn main() -> io::Result<()> {
    assert_eq!(U8_TO_HEX.len(), 256);

    // Get filename from arguments
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        eprintln!("For now, this app only takes one argument.");
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected one argument",
        ));
    }
    if args.len() < 2 {
        eprintln!("Please supply a file to open.");
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected one argument",
        ));
    }

    // Setup cursive
    let mut siv = Cursive::ncurses().expect("Failed to create a Cursive root.");

    siv.set_user_data(Data {
        hex_cache: HashMap::new(),
        file_path: String::new(),
    });

    // Initialize hex cache to so that conversions from hex to decimal can be looked
    // up in constant time. Also store file path.
    siv.with_user_data(|data: &mut Data| {
        for (i, hex) in U8_TO_HEX.iter().enumerate() {
            let index = i as u8;
            data.hex_cache.insert(hex, index);
        }

        data.file_path = args[1].to_string();
    });

    // Read input file to bytes, then convert to hex
    let byte_buffer: Vec<u8> = match util::read_as_byte_buffer(&args[1]) {
        Ok(b) => b,
        Err(why) => panic!("Couldn't read from file: {:?}", why),
    };

    let hex_values: Vec<&'static str> = byte_buffer
        .iter()
        .map(|byte| hex_conversion::convert_to_hex(*byte))
        .collect();

    main_view(&mut siv, &hex_values);

    siv.add_global_callback(Event::CtrlChar('g'), |s| goto_view(s));

    siv.run();

    Ok(())
}

struct Data {
    hex_cache: HashMap<&'static str, u8>,
    file_path: String,
}

fn main_view(siv: &mut Cursive, hex_values: &Vec<&str>) {
    let edit_area = HexArea::from(hex_values).with_id(HEX_AREA_ID);

    let dialog = Dialog::around(edit_area)
        .button("Save", |s| {
            let edit_area = s
                .find_id::<HexArea>(HEX_AREA_ID)
                .expect("Expected edit area to exist");

            let content = edit_area.get_content();

            let user_data = &s.user_data::<Data>().expect("Expected user data to exist");

            let buffer =
                match hex_conversion::convert_hex_str_to_bytes(content, &user_data.hex_cache) {
                    Ok(b) => b,
                    Err(_) => {
                        error_views::panic(s, "Invalid hex characters present.");
                        return;
                    }
                };

            if let Err(why) = util::write_bytes_to_file(&user_data.file_path, &buffer) {
                let message = format!("Couldn't write to file: {:?}", why);
                error_views::panic(s, &message);
            }

            s.add_layer(Dialog::text("File saved!").button("Ok", |s| {
                s.pop_layer();
            }));
        })
        .button("Quit", Cursive::quit)
        .full_screen();

    siv.add_layer(dialog);
}

fn goto_view(siv: &mut Cursive) {
    let layout = LinearLayout::vertical()
        .child(TextView::new("Enter a hexidecimal memory address:"))
        .child(
            EditView::new()
                .on_submit(goto_address)
                .max_content_width(8)
                .with_id(GOTO_ADDRESS_ID),
        );

    let dialog = Dialog::around(layout).button("Go", |s| {
        let address = s
            .call_on_id(GOTO_ADDRESS_ID, |view: &mut EditView| view.get_content())
            .expect("Expected edit view to exist");
        goto_address(s, &address);
    });

    siv.add_layer(dialog);

    fn goto_address(siv: &mut Cursive, address: &str) {
        siv.call_on_id(HEX_AREA_ID, |view: &mut HexArea| view.goto(address));
        siv.pop_layer();
    }
}
