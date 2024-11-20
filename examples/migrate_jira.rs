use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use chrono::{DateTime, NaiveDateTime};
use csv::ReaderBuilder;

use notionrs::{
    block::{Block, ParagraphBlock},
    client::Client,
    others::rich_text::{text::Text, RichTextAnnotations},
    page::{
        date::PageDatePropertyParameter, properties::PageProperty, PageDateProperty,
        PagePeopleProperty, PageRichTextProperty, PageSelectProperty, PageStatusProperty,
        PageTitleProperty,
    },
    Person, RichText, Select, User,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let token = std::env::var("NOTION_TOKEN").unwrap();
    let database_id = std::env::var("NOTION_DATABASE_ID").unwrap();
    let csv_file = std::env::var("CSV_FILE").unwrap();

    // Open and read CSV file
    let file = File::open(csv_file).expect("Failed to open CSV file");
    let reader = BufReader::new(file);
    let mut csv_reader = ReaderBuilder::new().has_headers(true).from_reader(reader);

    // Get headers
    let headers = csv_reader
        .headers()
        .expect("Failed to read CSV headers")
        .clone();

    // call Notion APi to get all members
    let client = Client::new().secret(token.clone());
    let members = client.list_users().recursive().send().await?;

    // create a map between member's name and member id
    let members_map = members
        .results
        .iter()
        .filter_map(|m| match m {
            User::Person(person) => person.name.clone().map(|name| (name, person.clone())),
            User::Bot(_) => None,
        })
        .collect::<HashMap<String, Person>>();

    // for each record
    for result in csv_reader.records() {
        let record = result?;
        let properties = create_page_properties(&headers, &record, &members_map);
        println!("{:?}", properties);

        let title = properties.get("Title").unwrap();
        // get description from record
        let description = record.get(headers.iter().position(|h| h == "Description").unwrap());

        // create page
        let client = Client::new().secret(token.clone());
        let blocks = vec![
            Block::Paragraph {
                paragraph: ParagraphBlock::from(title.to_string()),
            },
            Block::Paragraph {
                paragraph: ParagraphBlock::from(description.unwrap()),
            },
        ];
        let response = client
            .create_page()
            .database_id(database_id.clone())
            .properties(properties)
            .children(blocks)
            .send()
            .await?;
        println!("Successfully created page with ID: {}", response.id);
    }

    Ok(())
}
fn create_page_properties(
    headers: &csv::StringRecord,
    record: &csv::StringRecord,
    members_map: &HashMap<String, Person>,
) -> HashMap<String, PageProperty> {
    let mut properties = HashMap::new();

    for (i, header) in headers.iter().enumerate() {
        // Skip if header is not in our mapping
        let property_name = match header {
            "Summary" => "Title",
            "Issue key" => "JIRA Code",
            "Issue Type" => "Type",
            "Status" => "Status",
            "Priority" => "Priority",
            "Assignee" => "Assignee",
            "Reporter" => "Reporter",
            "Created" => "Created At",
            "Updated" => "Last Updated",
            "Fix versions" => "Fix Version",
            _ => continue, // Skip any unmapped columns
        };

        let value = record.get(i).unwrap_or_default();

        // Create property based on the column mapping
        match header {
            "Summary" => {
                properties.insert(
                    "Title".to_string(),
                    PageProperty::RichText(PageRichTextProperty {
                        rich_text: vec![RichText::Text {
                            text: Text::from(value),
                            annotations: RichTextAnnotations::default(),
                            plain_text: value.to_string(),
                            href: None,
                        }],
                        id: None,
                    }),
                );
            }
            "Issue key" => {
                properties.insert(
                    "ID".to_string(),
                    PageProperty::Title(PageTitleProperty::from(value)),
                );
            }
            "Fix Version" => {
                properties.insert(
                    property_name.to_string(),
                    PageProperty::RichText(PageRichTextProperty {
                        rich_text: vec![RichText::Text {
                            text: Text::from(value),
                            annotations: RichTextAnnotations::default(),
                            plain_text: value.to_string(),
                            href: None,
                        }],
                        id: None,
                    }),
                );
            }
            "Status" => {
                properties.insert(
                    "Status".to_string(),
                    PageProperty::Status(PageStatusProperty {
                        status: Select::from(value),
                        id: None,
                    }),
                );
            }
            "Issue Type" | "Priority" => {
                properties.insert(
                    property_name.to_string(),
                    PageProperty::Select(PageSelectProperty {
                        select: Some(Select::from(value)),
                        id: None,
                    }),
                );
            }
            "Created" | "Updated" => {
                properties.insert(
                    property_name.to_string(),
                    PageProperty::Date(PageDateProperty {
                        id: None,
                        date: Some(PageDatePropertyParameter {
                            start: Some(DateTime::from_naive_utc_and_offset(
                                NaiveDateTime::parse_from_str(value, "%d/%b/%y %I:%M %p").unwrap(),
                                chrono::FixedOffset::east_opt(7 * 3600).unwrap(),
                            )),
                            end: None,
                            ..Default::default()
                        }),
                    }),
                );
            }
            "Assignee" => {
                let person = members_map
                    .get(value)
                    .map(|member| User::Person(member.clone()));

                match person {
                    Some(user) => {
                        properties.insert(
                            "Assignee".to_string(),
                            PageProperty::People(PagePeopleProperty {
                                people: vec![user],
                                id: None,
                            }),
                        );
                    }
                    _ => {
                        properties.insert(
                            "Assignee".to_string(),
                            PageProperty::People(PagePeopleProperty {
                                people: vec![],
                                id: None,
                            }),
                        );
                        properties.insert(
                            "Assigned To".to_string(),
                            PageProperty::RichText(PageRichTextProperty {
                                id: None,
                                rich_text: vec![RichText::Text {
                                    text: Text::from(value),
                                    annotations: RichTextAnnotations::default(),
                                    plain_text: value.to_string(),
                                    href: None,
                                }],
                            }),
                        );
                    }
                }
            }
            "Reporter" => {
                let person = members_map
                    .get(value)
                    .map(|member| User::Person(member.clone()));

                match person {
                    Some(user) => {
                        properties.insert(
                            "Assignee".to_string(),
                            PageProperty::People(PagePeopleProperty {
                                people: vec![user],
                                id: None,
                            }),
                        );
                    }
                    _ => {
                        properties.insert(
                            "Assignee".to_string(),
                            PageProperty::People(PagePeopleProperty {
                                people: vec![],
                                id: None,
                            }),
                        );
                        properties.insert(
                            "Assigned To".to_string(),
                            PageProperty::RichText(PageRichTextProperty {
                                id: None,
                                rich_text: vec![RichText::Text {
                                    text: Text::from(value),
                                    annotations: RichTextAnnotations::default(),
                                    plain_text: value.to_string(),
                                    href: None,
                                }],
                            }),
                        );
                    }
                }
            }
            "Fix versions" => {
                properties.insert(
                    "Fix Version".to_string(),
                    PageProperty::RichText(PageRichTextProperty {
                        rich_text: vec![RichText::Text {
                            text: Text::from(value),
                            annotations: RichTextAnnotations::default(),
                            plain_text: value.to_string(),
                            href: None,
                        }],
                        id: None,
                    }),
                );
            }
            _ => unreachable!(), // We've already filtered out unmapped columns
        };
    }

    // there are some columns with same title "Inward issue link (Blocks)"
    // try to read all values and merge into a string

    let inward_issue_links = get_combined_values(&record, &headers, "Inward issue link (Blocks)");
    let outward_issue_links = get_combined_values(&record, &headers, "Outward issue link (Blocks)");
    properties.insert(
        "Related Issues".to_string(),
        PageProperty::RichText(PageRichTextProperty {
            rich_text: vec![RichText::Text {
                text: Text::from(outward_issue_links.clone()),
                annotations: RichTextAnnotations::default(),
                plain_text: outward_issue_links,
                href: None,
            }],
            id: None,
        }),
    );
    properties.insert(
        "Block Issues".to_string(),
        PageProperty::RichText(PageRichTextProperty {
            rich_text: vec![RichText::Text {
                text: Text::from(inward_issue_links.clone()),
                annotations: RichTextAnnotations::default(),
                plain_text: inward_issue_links,
                href: None,
            }],
            id: None,
        }),
    );

    properties
}
fn get_combined_values(
    record: &csv::StringRecord,
    headers: &csv::StringRecord,
    column_name: &str,
) -> String {
    headers
        .iter()
        .enumerate()
        .filter(|(_, h)| h == &column_name)
        .filter_map(|(i, _)| record.get(i))
        .filter(|&v| !v.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}
