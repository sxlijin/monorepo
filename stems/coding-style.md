# Coding Style Guide

## Rust Code Style

### Control Flow

#### Avoid Nested Conditionals
- avoid nesting conditionals; try to use guard clauses instead.

    Bad:

        fn process_data(data: Option<Data>) -> Result<Output, Error> {
            if let Some(data) = data {
                if data.is_valid() {
                    if let Some(processed) = data.process() {
                        if processed.is_ready() {
                            return Ok(processed.to_output());
                        } else {
                            return Err(Error::NotReady);
                        }
                    } else {
                        return Err(Error::ProcessingFailed);
                    }
                } else {
                    return Err(Error::InvalidData);
                }
            } else {
                return Err(Error::NoData);
            }
        }

    Good:

        fn process_data(data: Option<Data>) -> Result<Output, Error> {
            let Some(data) = data else {
                return Err(Error::NoData);
            }
            
            // Guard clause: early return for invalid data
            if !data.is_valid() {
                return Err(Error::InvalidData);
            }
            
            // Guard clause: early return for processing failure
            let processed = data.process().ok_or(Error::ProcessingFailed)?;
            
            // Guard clause: early return for not ready
            if !processed.is_ready() {
                return Err(Error::NotReady);
            }
            
            // Success case
            Ok(processed.to_output())
        }
