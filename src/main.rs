use chrono::Datelike as _;
use dotenvy::dotenv;
use libsixclient::{
    fetch_cal_base, fetch_cal_path, get_calevents_today, get_client, get_curr_or_next_calevent,
    is_valid_login_cookie, now_with_offset, tandai_hadir,
};

fn main() {
    dotenv().expect("Unable to load .env file");
    let cookie = ::std::env::var("LOGIN_COOKIE").expect("No login cookie found in env");
    let nim = ::std::env::var("STUDENT_NIM")
        .expect("No student NIM found in env")
        .parse::<u32>()
        .expect("Student NIM in env is not valid u32");

    let client = get_client(&cookie);

    let now = now_with_offset();
    let month = now.month();

    // Aug - Jan = Sem 1
    // Feb - Jun = Sem 2
    let semester = if 8 <= month || month == 1 { 1 } else { 2 };
    // If year is 2027 but semester is 2, then actual SIX calendar year is 2026
    let year = (now.year() as u32) + 1 - semester;

    if !is_valid_login_cookie(&client) {
        println!("Cookie for login is invalid! Get a new cookie!");
        return;
    }

    let html = fetch_cal_base(&client, nim, year, semester);

    let calevs = get_calevents_today(&client, &html);
    let curr_or_next = get_curr_or_next_calevent(&calevs);

    if let Some(event) = curr_or_next {
        println!("{}-{}", event.start, event.end);
        let html = fetch_cal_path(&client, &event.linkpertemuan);
        tandai_hadir(&client, &html);
    } else {
        println!("No current or next event can be found for the day");
    }
}
