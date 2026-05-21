pub fn query_pairs_from_req(req: &salvo::Request) -> Vec<(String, String)> {
    req.queries()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}
