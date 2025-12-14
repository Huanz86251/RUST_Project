# Project Proposal: Rust Web Crawler with Data Analysis
## Group Members

| Role | Name | Student ID | GitHub ID |
|------|------|-------------|-----------|
| **Member A** | Zihao Gong | 1005036916 | [Zihao1121](https://github.com/Zihao1121) |
| **Member B** | Shiming Zhang | 1011821129 | [Ming031121](https://github.com/Ming031121) |
| **Member C** | Zixuan Huang | 1006288376 | [Huanz86251](https://github.com/Huanz86251) |

## Motivation
With the popularization of the Internet, today’s web is becoming complex and vast; it is difficult for users to efficiently find the information they need. Traditional crawler scripts struggle to help users find trustworthy, relevant, and up-to-date evidence on target topics. Furthermore, a major industry challenge with large language model technology is reducing model hallucinations. High-quality evidence-grounded retrieval data is key to addressing this issue, and low-quality crawler data can lead to misjudgments in large language models. We aim to develop a Rust-based crawler pipeline that integrates web crawling with the retrieval and evidence-extraction side of RAG (we will add generation if there is enough time left). This pipeline captures product information and user reviews from e-commerce platforms, cleans and deduplicates raw data, normalizes the storage of numeric facts, such as price, weight, and rating, builds a queryable semantic index, and provides answers and references to user queries in a traceable manner with URL, timestamp, and evidence sentence.  

For subjective comments, we use ANN retrieval and cross-encoder re-ranking to return the most relevant product reviews and descriptions. And for specific values, we will search the database, return solid data, and generate corresponding figures, such as historical price trends. This ensures that mathematical facts are separated from language facts, preventing users and LLMs from getting mixed information.  

In the Rust ecosystem, crawling and parsing scripts are relatively mature, but integrated pipelines and reusable components, from crawling to semantic indexing, evidence extraction, and re-ranking, and finally generating evidence-based answers, are still relatively scarce. Most retrieval scripts focus more on minimal vector-retrieval demos and ignore the complete pipeline of data crawling, storage, and extraction. We hope to leverage Rust's speed and safety to quickly convert web pages into searchable and traceable evidence, which may benefit a lot of people. Especially for the large language models field, Rust's high runtime speed can increase retrieval and preprocessing throughput, which effectively reduces large language models' end-to-end latency during inference. For human users, this can also reduce their search time and provide the most relevant data.

---

## Objective and Key Features

The project’s objective is to design and implement a high-performance, memory-safe and scalable web crawler that follows a data processing pipeline and presents the result through an interactive text user interface. Also, the collected and cleaned data will serve as high-quality retrieval material for future Retrieval-Augmented Generation (RAG) systems, providing structured and reliable input for LLM-based applications.  

The tool’s functions include historical price visualization, forecasting potential price fluctuations, finding the most relevant reviews for individual products, cross-platform comparisons among similar items, and rating and review similarity analysis to assess product popularity and overall user satisfaction.  

To achieve this objective, the system incorporates several performance-oriented features.

### Asynchronous Web Crawling
We will implement concurrent, non-blocking data collection using Rust’s async/await model and stackless coroutines to efficiently fetch data from multiple e-commerce websites. The `Tokio` runtime will manage asynchronous task scheduling, while `Reqwest` will handle HTTP requests for retrieving HTML content.  

To ensure efficient data collection and minimize waiting time, the crawler utilizes asynchronous operations implemented with stackless coroutines. This approach enables the program to handle numerous concurrent network requests without blocking, significantly accelerating the overall data retrieval process.

### Data Cleaning and Analysis Using Rust
We will use the `Scraper` crate to parse HTML documents and extract relevant information while filtering out advertisements, scripts, and other irrelevant content. The data cleaning module will standardize formats (e.g., price strings, currency symbols), handle missing or inconsistent values, and ensure data integrity for accurate analysis.  

Instead of relying on external platforms such as Azure or Python-based models, this project performs data cleaning and analysis entirely in Rust, which guarantees no null pointers, buffer overflows, or data races through its ownership and borrowing system. This design choice significantly reduces the likelihood of system crashes and enhances the overall stability and reliability of the tool. Compared to Python, which depends on a garbage collector and can suffer from runtime overhead or concurrency limitations, Rust offers deterministic performance and thread-safe parallelism. Unlike Azure’s managed cloud pipelines, Rust provides full local control over data processing with minimal dependency and latency. This design choice ensures greater efficiency, stability, and reliability across the entire data pipeline.

### Language Vector Index
After capturing and storing user reviews and product descriptions, we will load the Transformer model using `tch-rs` and put the text into the model. We will extract the model's last hidden state layer as the text vector and store it. When a user asks a question, we utilize `hnsw-rs` to perform an Approximate Nearest Neighbour (ANN) search to find the top 50 most relevant sentences. Then, we use Hugging Face to export a quantized cross-encoder model and load it with `ONNX Runtime`. The cross-encoder can perform a more fine-grained re-ranking of the top 50 candidate sentences based on the user's question.

### Text User Interface
We will develop an interactive text-based user interface (TUI) that allows users to perform various analyses directly within the terminal. This interface will be built using the `Ratatui` crate, providing an intuitive and efficient way to explore data, visualize results, and monitor the crawling and analysis process in real time.  

To enhance usability and accessibility, the project employs a text-based user interface (TUI) for presenting essential information and analytical charts directly within the terminal. Compared to graphical components, the TUI uses characters and colour blocks. This lightweight interface enhances efficiency by minimizing resource usage and ensuring both human-readability and interpretability by large language models, facilitating potential future integration.

### Novelty
To make full use of Rust’s strengths, this project is designed as a complete, end-to-end data processing pipeline that supports both developers and general users. While end users may not directly perceive the benefits of Rust’s high performance and memory safety, these features are crucial for building reliable and scalable Retrieval-Augmented Generation (RAG) systems. Although the project itself does not perform text generation, it is designed to integrate seamlessly with LLM-based applications, serving as a robust foundation for future intelligent extensions. By exploring this connection, the project bridges traditional data engineering with modern AI workflows, filling a practical gap in the current Rust ecosystem.

---

## Project Timeline

### Week 1–2: Crawling and Data Cleaning

**Member A**
- Build the asynchronous crawler using `reqwest` and `tokio`.
- Ensure `robots.txt` compliance and rate limiting.

**Member B and C**
- Parse HTML and extract structured product attributes.
- Normalize data into a unified schema (price normalization, category standardization, etc.).
- If cleaning proves to be a large workload, all three members will collaborate on this stage to ensure consistency and robustness.

### Week 3–4: Data Analysis

**Member A**
- Implement historical price visualization and predictive modelling to forecast potential price fluctuations based on temporal trends and market behaviour.

**Member B**
- Develop a review embedding and similarity analysis module that vectorizes user reviews to identify semantically related opinions and product feedback patterns.

**Member C**
- Conduct cross-platform product comparison and rating distribution analysis to evaluate consistency in pricing, popularity, and user satisfaction across different e-commerce sources.

### Week 5: Integration and Advanced Feature Development

**Member A**
- Start developing a UI interface to enhance the Ratatui interface with multiple tabs: overview, price insights, and rating/review analysis.
- Add navigation features and CSV export.

**Member B and C**
- Finalize duplicate product detection and integrate it into the analysis layer.
- Produce finalized analysis results and structured outputs, then coordinate closely with Member A so that these results can be displayed in the user interface.

### Week 6: Testing and Final Delivery

**All Members**
- Collaboratively test all modules, optimize performance, and ensure graceful error handling.
- Prepare documentation, sample outputs, and demonstration runs to showcase the system.



