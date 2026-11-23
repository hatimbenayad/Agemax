/// Determine the platform-specific default named pipe / socket endpoint.
pub fn get_default_pipe_name() -> String {
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "default".to_string());

    #[cfg(windows)]
    {
        format!(r"\\.\pipe\agemax-{}", username)
    }

    #[cfg(not(windows))]
    {
        format!("/tmp/agemax-{}.sock", username)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipe_name_format() {
        let name = get_default_pipe_name();
        #[cfg(windows)]
        assert!(name.starts_with(r"\\.\pipe\agemax-"));
        #[cfg(not(windows))]
        assert!(name.starts_with("/tmp/agemax-"));
    }
}
