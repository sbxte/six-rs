use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Datelike as _, FixedOffset, NaiveTime, Utc};
use reqwest::Url;
use reqwest::blocking::{Client, ClientBuilder};
use reqwest::cookie::Jar;
use scraper::{Html, Selector};

const SIX_URL: &str = "https://six.itb.ac.id";

pub fn get_client(khongguan: &str) -> Client {
    let cookie_store = Jar::default();
    cookie_store.add_cookie_str(
        &format!("khongguan={khongguan}"),
        &"https://six.itb.ac.id/".parse::<Url>().unwrap(),
    );
    cookie_store.add_cookie_str(
        "nissin=ms365",
        &"https://six.itb.ac.id/".parse::<Url>().unwrap(),
    );
    cookie_store.add_cookie_str(
        "_locale=en",
        &"https://six.itb.ac.id/".parse::<Url>().unwrap(),
    );
    let provider = Arc::new(cookie_store);
    let client = ClientBuilder::new()
        .cookie_provider(provider)
        .user_agent("Mozilla/5.0")
        .build()
        .unwrap();
    client
}

pub fn is_valid_login_cookie(client: &Client) -> bool {
    let selector = Selector::parse("#login").unwrap();
    client
        .get(SIX_URL)
        .send()
        .map(|r| r.text())
        .flatten()
        .map(|html| {
            let html = Html::parse_document(&html);
            html.select(&selector).next().is_none()
        })
        .unwrap_or(false)
}

pub fn fetch_cal_base(client: &Client, nim: u32, year: u32, semester: u32) -> Html {
    let body = client
        .execute(
            client
                .get(format!(
                    "{SIX_URL}/app/mahasiswa:{nim}+{year}-{semester}/kelas/jadwal/mahasiswa"
                ))
                .build()
                .unwrap(),
        )
        .unwrap()
        .text()
        .unwrap();
    Html::parse_document(&body)
}

pub fn fetch_cal_path(client: &Client, path: &str) -> Html {
    let body = client
        .execute(client.get(format!("{SIX_URL}{path}")).build().unwrap())
        .unwrap()
        .text()
        .unwrap();
    Html::parse_document(&body)
}

// Calendar Month Year
pub fn split_cal_moyr(s: &str) -> (&str, u64) {
    let (month, year) = s
        .split_once(" ")
        .expect("Unable to split month and year from calendar");
    let year: u64 = year.parse().unwrap();
    (month, year)
}

pub fn get_cal_month_paths(html: &Html) -> HashMap<String, &str> {
    let month_selector =
        Selector::parse("div.container ul.nav.nav-tabs.hidden-print li a").unwrap();

    let mut map = HashMap::new();

    for calmonth in html.select(&month_selector) {
        let path = calmonth.attr("href").unwrap();
        let cal_moyr = calmonth.inner_html();
        map.insert(cal_moyr, path);
    }

    map
}

pub const fn get_timezone_offset() -> FixedOffset {
    FixedOffset::east_opt(7 * 3600).unwrap()
}

pub fn now_with_offset() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&get_timezone_offset())
}

#[derive(Debug, Clone)]
pub struct CalEvent {
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub linkpertemuan: String,
}

pub fn get_cal_day_schedule(html: &Html, day: u32) -> Vec<CalEvent> {
    let selector =
        Selector::parse("div.container div.panel.panel-default div.table-responsive table.table.table-striped.table-bordered tbody tr td")
            .unwrap();
    let div_selector = Selector::parse("div").unwrap();
    let class_event_link_sel = Selector::parse("div div a.linkpertemuan").unwrap();
    let class_event_time_sel = Selector::parse("small").unwrap();

    let mut sched = vec![];
    for td in html.select(&selector) {
        let x = td.select(&div_selector).nth(0).unwrap().inner_html();
        let x = x.trim();
        let parsed_day = if x.find('\n').is_some() {
            let (parsed_day, _) = x.split_once('\n').unwrap();
            parsed_day
                .trim()
                .parse::<u32>()
                .expect("Unable to parse day from calendar")
        } else {
            x.parse::<u32>().expect("Unable to parse day from calendar")
        };

        if parsed_day != day {
            continue;
        }

        for event_a in td.select(&class_event_link_sel) {
            // Link pertemuan

            let linkpertemuan = event_a
                .attr("data-url")
                .expect("Unable to get data-url from cal event")
                .to_string();

            // Time range
            let small = event_a
                .select(&class_event_time_sel)
                .nth(0)
                .expect("Unable to get cal event time html");
            let inner = small.inner_html();
            let time_range_str = inner
                .trim()
                .split_once('\n')
                .map(|(s, _)| s.trim())
                .expect("Unable to get time range from calendar event");
            let (time_start_str, time_end_str) = time_range_str
                .split_once('-')
                .expect("Unable to split calendar event time range into start and end");
            let time_start = NaiveTime::parse_from_str(time_start_str, "%H:%M")
                .expect("Unable to parse cal event time start");
            let time_end = NaiveTime::parse_from_str(time_end_str, "%H:%M")
                .expect("Unable to parse cal event time end");

            sched.push(CalEvent {
                start: time_start,
                end: time_end,
                linkpertemuan,
            });
        }
    }

    sched
}

pub fn get_calevents_today(client: &Client, base_cal: &Html) -> Vec<CalEvent> {
    let now = now_with_offset();
    let curr_mon_str = now.format("%b %Y").to_string();

    let mopaths = get_cal_month_paths(&base_cal);
    let curr_mo_path = mopaths[&curr_mon_str];

    let day = now.day();
    let curr_mo_html = fetch_cal_path(client, curr_mo_path);
    get_cal_day_schedule(&curr_mo_html, day)
}

pub fn get_curr_or_next_calevent(calevents: &[CalEvent]) -> Option<&CalEvent> {
    let time_now = now_with_offset().time();

    // NOTE: We will assume that there are absolutely no conflicting class schedules
    calevents.iter().skip_while(|ce| ce.end < time_now).nth(0)
}

pub fn tandai_hadir(client: &Client, html: &Html) {
    let btn_selector =
        Selector::parse("form div.text-center button#form_hadir.btn.btn-sm.btn-primary").unwrap();
    let form_selector = Selector::parse("form").unwrap();
    let returnto_selector = Selector::parse("form input#form_returnTo.form-control").unwrap();
    let token_selector = Selector::parse("form input#form__token.form-control").unwrap();

    if html.select(&btn_selector).next().is_none() {
        // No tandai hadir button
        println!("No tandai hadir button found");
        return;
    }

    let action_path = html
        .select(&form_selector)
        .next()
        .and_then(|e| e.attr("action"))
        .expect("Unable to get tandai hadir form");

    // action_path is relative (e.g. "/app/mahasiswa:.../mhs/2021183827?returnTo=..."),
    // same as fetch_cal_path's `path` param — just prepend the base.
    let action_url = format!("{SIX_URL}{action_path}");

    let return_to = html
        .select(&returnto_selector)
        .next()
        .and_then(|e| e.attr("value"))
        .expect("Unable to get form_returnTo value for tandai hadir form");

    let token = html
        .select(&token_selector)
        .next()
        .and_then(|e| e.attr("value"))
        .expect("Unable to get form__token value for tandai hadir form");

    let resp = client
        .post(action_url) // returnTo is already in the query string from action_path
        .form(&[
            ("form[hadir]", ""),
            ("form[returnTo]", return_to),
            ("form[_token]", token),
        ])
        .send();

    match resp {
        Ok(r) => {
            println!("Successfully tandai hadir! status: {}", r.status())
        }
        Err(e) => {
            println!("Failed to tandai hadir: {}", e);
        }
    }
}
