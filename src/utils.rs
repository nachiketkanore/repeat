use serde::Serialize;

pub fn print_struct_as_json<T>(data: &T)
where
    T: Serialize,
{
    let json_output = serde_json::to_string_pretty(data)
        .expect("Failed to serialize struct to JSON. This should not happen.");
    println!("{}", json_output);
}
