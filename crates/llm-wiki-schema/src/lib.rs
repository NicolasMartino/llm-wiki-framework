pub fn nothing() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_is_callable() {
        nothing();
    }
}
