use kuchiki::traits::*;
use std::borrow::Borrow;
use std::collections::{HashMap};
use regex::{Regex};
use unicode_segmentation::UnicodeSegmentation;
use colored::*;
use clap::{Arg, App, SubCommand};

fn word_analysis(title: &str) -> Result<(), Box<std::error::Error>> {
    println!("{}{}...", "Fetching ".bold(), title);
    let page_url = title;
    // TODO replace spaces with underscores

    let base_url = url::Url::parse("https://en.wikipedia.org/api/rest_v1/page/html/").unwrap();
    let mut resp = reqwest::get(base_url.join(page_url).unwrap())?;
    let resp_html = resp.text()?;
    let doc = kuchiki::parse_html().one(resp_html);
    
    let mut nodes_to_delete = vec![];
    // NOTE: these must be placed in a vec because if we detach it in the loop
    // the loop only runs once, due to unintended behaviour!
    for node_ref in doc.select("section > p > sup").unwrap() {
        nodes_to_delete.push(node_ref);
    }
    for n in nodes_to_delete {
        n.as_node().detach();
    }
    let mut text: String = "".to_string();
    
    for node_ref in doc.select("section > p").unwrap() {
        //print
        let p = node_ref.text_contents();
        text += &p;
        // let re = regex::Regex::new(r" [\[\]()]").unwrap();
        // let words = re.replace_all(s, re);
    }
    println!("{}", text);
    let stats = word_count(text);
    for (k, count) in stats.word_counts.iter() {
        let word_variants = stats.word_variants.get(k).unwrap();
        let word_entry = word_variants.join("/");
        println!("{}: {}", word_entry, count);
    }
    Ok(())
}

struct TextStatistics {
    word_counts: HashMap<String, u32>,
    word_variants: HashMap<String, Vec<String>>,
}
fn word_count(text: String) -> TextStatistics {
    let mut stats = TextStatistics {
        word_counts: HashMap::new(),
        word_variants: HashMap::new(),
    };
    // Latin
    let latin = r"[A-Za-zÀ-ÖØ-öø-ÿ]";
    // match latin/dash/apostrophe
    let re = Regex::new(&format!(r"{0}[{0}'\-]+", latin)).unwrap();
    for cap in re.captures_iter(&text) {
        let word = cap[0].to_string();
        print!("{} ", word);
        let lower = word.to_lowercase(); // unicode supported
        
        let word_count = stats.word_counts.entry(lower.clone()).or_insert(0);
        *word_count += 1;

        let variants = stats.word_variants.entry(lower).or_insert(Vec::new());
        if !variants.contains(&word) {
            variants.push(word);
        }
    }

    stats
}

/// Deletes things contained in parentheses
/// as the first wikilink we want must be "in the main text", meaning not parenthesised.
/// We keep parentheses inside tag attributes however.
/// (hello)hello changes to hello, but <a href="hello_(hello)"></a> is unchanged.
fn delete_parentheses<S: Into<String>>(input_str: S) -> String {
    let mut parenth_level = 0;
    let mut tag_level = 0;
    let mut result: String = "".to_string();
    let s: String = input_str.into();
    for c in s.graphemes(true) {
        if parenth_level <= 0 {
            if c == "<" {
                tag_level += 1;
            }
            if c == ">" {
                tag_level -= 1;
            }
        }
        if tag_level <= 0 {
            if c == "(" {
                parenth_level += 1;
            }
            // note: this if clause must be first otherwise there will be dangling )
            if parenth_level == 0 {
                result += c;
            }
            if c == ")" {
                parenth_level -= 1;
            }
        }
        else {
            result += c;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::delete_parentheses;
    #[test]
    fn parenth1() {
        assert_eq!(delete_parentheses("hello (hello (hello) hello) hello"), "hello  hello")
    }
    #[test]
    fn parenth2() {
        assert_eq!(delete_parentheses("()"), "")
    }
    #[test]
    fn parenth3() {
        assert_eq!(delete_parentheses("<a href='hello_(hello)'>(hello)hello</a>"), "<a href='hello_(hello)'>hello</a>")
    }
    #[test]
    fn parenth4() {
        assert_eq!(delete_parentheses("<a>(<b>)<b>(f<a>jjj</a>>)hello"), "<a><b>hello")
    }
}

fn follow_first_links(initial_page: &str, final_page: &str) -> Result<(), Box<std::error::Error>> {
    let client = reqwest::Client::new();

    println!("{}", ">> Following first wikilinks of each page".bright_green());
    let mut count = 0;
    let mut current = format!("{}", initial_page);
    // the rest_v1 html has lots of additional metadata
    let base_url = url::Url::parse("https://en.wikipedia.org/api/rest_v1/page/html/").unwrap();

    let mut visited_links: Vec<String> = vec![];

    loop {
        if current == final_page {
            println!("{}", format!("Reached {} in {} clicks!", final_page, count).green().bold());
            return Ok(());
        }
        println!("{}{}...", "Visiting ".bold(), current);
        if visited_links.contains(&current) {
            println!("{}", "Cycle detected!".green().bold());
            return Ok(());
        }
        let url = base_url.join(&current).unwrap();
        let mut resp = client.get(url).send()?;
        if !resp.status().is_success() {
            println!("{}", format!("Page {} nonexistent!", current).red().bold());
            return Ok(());
        }
        visited_links.push(current.clone());
        let resp_html = delete_parentheses(resp.text()?);

        //et mut resp_html = r"<html><body>nihaoma?</body></html>";
        let doc = kuchiki::parse_html().one(resp_html);

        let mut link_found = false;

        for node_ref in doc.select("section > p > a[rel='mw:WikiLink']").unwrap() {
            let node = node_ref.as_node();
            // millions of unwrap()s and borrow()s
            let node_el = node.as_element().unwrap().borrow();
            let attrs = node_el.attributes.borrow();

            current = attrs.get("href").borrow().unwrap().to_string()[2..].to_string();
            //println!("{}", current);

            if !current.contains("Help:") && !current.contains("Template:") {
                link_found = true;
                break;
            }
        }
        if !link_found {
            println!("{}", format!("Deadend: No wikilink found on {}!", current).red().bold());
            break;
        }

        count += 1;
        if count >= 500 {
            break;
        }
    }
    Ok(())
}
fn main() {
    let app = App::new("The Wiki Scraper")
        .version("0.1.0")
        .before_help("The best command-line Wikipedia tool ever!!1!")
        .subcommand(SubCommand::with_name("analysis")
            .about("Analyzes word counts of an article")
            .arg(Arg::with_name("title").required(true)
                .help("The title of the page to analyze")))
        .subcommand(SubCommand::with_name("first-link")
            .about(
            "Clicks the first link of the article repeatedly to try to get to the desired destination article, like the Wikipedia Philosophy game")
            .arg(Arg::with_name("start")
                .required(true)
                .help("The initial wikipage"))
            .arg(Arg::with_name("end")
                .help("The destination wikipage, defaults to Philosophy")))
        .get_matches();

    println!("{}{}{}", ">>>>> ",
        "THE WIKIPEDIA SCRAPER".cyan().bold(),
        " <<<<<");
    
    if let Some(matches) = app.subcommand_matches("first-link") {
        let start = matches.value_of("start").unwrap();
        let end = matches.value_of("end").unwrap_or("Philosophy");
        match follow_first_links(start, end) {
            Ok(_) => (),
            Err(e) => println!("{}\n{}", "AIYAA! an error:".red().bold(), e)
        };
    }
    if let Some(matches) = app.subcommand_matches("analysis") {
        match word_analysis(matches.value_of("title").unwrap()) {
            Ok(_) => (),
            Err(e) => println!("{}\n{}", "AIYAA! an error:".red().bold(), e)
        };
    }
}