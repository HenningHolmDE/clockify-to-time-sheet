# Clockify to time sheet

A small Rust application to create a custom monthly time sheet from time entries
tracked through [Clockify](https://clockify.me), an online time tracking system.
The application retrieves the time entries for a given month through the
Clockify REST API and generates a CSV file formatted for the needs of the time
sheet template.

## Usage

Copy the `config_template.toml` to `config.toml` and provide the configuration
values (e.g. Clockify API key) required for the application.

## Architecture

Most of the functionality of the application is divided into three modules: 
- The `clockify` module is responsible for querying the Clockify REST API and
  returning a `Vec<TimeEntry>` for further processing.
- Through the `transform` module, these time entries are transformed into the
  entries required for the time sheet. This step merges subsequent entries of
  the same task while keeping track of the break times in between. This way, the
  amount of entries in the time sheet is kept short.
- Finally, the `writer` module generates a CSV file formatted according to the
  requirements for the time sheet.
