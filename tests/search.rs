mod integration_tests {

    use notionrs::to_json::ToJson;

    // # --------------------------------------------------------------------------------
    //
    // search
    //
    // # --------------------------------------------------------------------------------

    #[tokio::test]
    #[serial_test::serial]
    async fn search() -> Result<(), notionrs::error::Error> {
        dotenvy::dotenv().ok();

        let client = notionrs::client::Client::new();

        let request = client
            .search()
            .query("My Title")
            .filter_page()
            .sort_timestamp_asc();

        let response = request.send().await?;

        println!("{}", response.to_json());

        Ok(())
    }
}
