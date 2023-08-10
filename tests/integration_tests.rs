use itertools::intersperse;
use std::fs::create_dir;
use std::fs::OpenOptions;
use std::io::BufRead;
use std::io::Cursor;
use std::path::Path;
use std::process::Command;
use uuid::Uuid;

const ONE: &str = env!("CARGO_BIN_EXE_one");
const TWO: &str = env!("CARGO_BIN_EXE_two");
const SAMPLES: usize = 200;

fn find_last_space(slice: &[u8]) -> usize {
    for i in (0..slice.len()).rev() {
        if slice[i] == b' ' {
            return i;
        }
    }
    panic!("Missing space");
}

// Get an average profile across many profiles for a single-threaded application.
//
// This presumes ordering of spans between profiles matches and they all have the same spans, this
// could support profiles with different spans and multithreading but it would be more complex.
fn average_synchronous_profiles<P: AsRef<Path>>(profiles: &[P]) -> String {
    let mut iter = profiles.iter();
    let first_profile = iter.next().expect("Need at least 1 profile");
    let file = OpenOptions::new().read(true).open(first_profile).unwrap();
    let reader = std::io::BufReader::new(file);
    let mut lines = reader
        .lines()
        .map(Result::unwrap)
        .map(|line| {
            let bytes = line.as_bytes();
            let i = find_last_space(bytes);
            (
                std::str::from_utf8(&bytes[..=i]).unwrap().to_string(),
                std::str::from_utf8(&bytes[i + 1..])
                    .unwrap()
                    .parse::<u128>()
                    .unwrap(),
            )
        })
        .collect::<Vec<_>>();

    for profile in profiles {
        let file = OpenOptions::new().read(true).open(profile).unwrap();
        let reader = std::io::BufReader::new(file);
        for (n, line) in reader.lines().map(Result::unwrap).enumerate() {
            let bytes = line.as_bytes();
            let i = find_last_space(bytes);
            let count = std::str::from_utf8(&bytes[i + 1..])
                .unwrap()
                .parse::<u128>()
                .unwrap();
            lines[n].1 += count;
        }
    }

    let output_str = intersperse(
        lines
            .into_iter()
            .map(|(a, b)| format!("{a}{}", b / (profiles.len() as u128))),
        String::from("\n"),
    )
    .collect::<String>();
    output_str
}

#[test]
fn pair() {
    let run_dir = format!("./tmp/{}", Uuid::new_v4());
    println!("run_dir: {run_dir}");
    create_dir(&run_dir).unwrap();

    // Get first profile
    let one_dir = format!("{run_dir}/one");
    create_dir(&one_dir).unwrap();
    let files = (0..SAMPLES)
        .map(|i| {
            let path = format!("{one_dir}/{i}.prof");
            let _output = Command::new(ONE).arg(&path).output().unwrap();
            path
        })
        .collect::<Vec<_>>();
    let one_avg_profile = average_synchronous_profiles(&files);

    // Get second profile
    let two_dir = format!("{run_dir}/two");
    create_dir(&two_dir).unwrap();
    let files = (0..SAMPLES)
        .map(|i| {
            let path = format!("{two_dir}/{i}.prof");
            let _output = Command::new(TWO).arg(&path).output().unwrap();
            path
        })
        .collect::<Vec<_>>();
    let two_avg_profile = average_synchronous_profiles(&files);

    // To properly generate the differential flamegraph the spans need to match, presently since
    // these are 2 separate binaries they do not match, we update them to match.
    let one_avg_profile = one_avg_profile
        .replace("one::", "three::")
        .replace("one.rs", "three.rs");
    let two_avg_profile = two_avg_profile
        .replace("two::", "three::")
        .replace("two.rs", "three.rs");

    println!("one_avg_profile:\n{one_avg_profile}\n");
    println!("two_avg_profile:\n{two_avg_profile}\n");

    // Get difference profile
    let mut diff = Vec::<u8>::new();
    inferno::differential::from_readers(
        inferno::differential::Options {
            // The performance difference in our binaries is 3.5ms this is relatively small as a
            // percentage (due to the startup time dominating) so to maximize visibility we normalize.
            normalize: true,
            strip_hex: false,
        },
        Cursor::new(one_avg_profile.as_bytes()),
        Cursor::new(two_avg_profile.as_bytes()),
        &mut diff,
    )
    .unwrap();

    println!(
        "difference profile:\n{}\n",
        std::str::from_utf8(&diff).unwrap()
    );

    // Convert difference profile to flamegraph
    let buff = Cursor::new(diff);
    let mut flamegraph = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open("./pair.svg")
        .unwrap();
    inferno::flamegraph::from_reader(
        &mut inferno::flamegraph::Options::default(),
        buff,
        flamegraph,
    )
    .unwrap();
}
