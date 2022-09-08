/// Normalizes the return of `read_line()` in the context of a CLI application
pub fn fix_line_issues(mut line: String) -> std::io::Result<String> {
    if !line.ends_with('\n') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "unexpected end of file",
        ));
    }

    // Remove the \n from the line.
    line.pop();

    // Remove the \r from the line if present
    if line.ends_with('\r') {
        line.pop();
    }

    if line.contains("") {
        let vec: Vec<&str> = line.split("").collect();
        line = match vec.last() {
            None => String::new(),
            Some(i) => i.to_string()
        }
    }

    Ok(line)
}
