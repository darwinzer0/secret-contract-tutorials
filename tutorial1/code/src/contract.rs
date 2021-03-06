use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdError,
    StdResult, Storage,
};
use std::convert::TryFrom;
use crate::msg::{HandleMsg, InitMsg, QueryMsg, HandleAnswer, QueryAnswer};
use crate::state::{load, may_load, save, State, Reminder, CONFIG_KEY};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let max_size = match valid_max_size(msg.max_size) {
        Some(v) => v,
        None => return Err(StdError::generic_err("Invalid max_size. Must be in the range of 1..65535."))
    };

    let config = State {
        max_size,
        reminder_count: 0_u64,
    };

    save(&mut deps.storage, CONFIG_KEY, &config)?;
    Ok(InitResponse::default())
}

// limit the max message size to values in 1..65535
fn valid_max_size(val: i32) -> Option<u16> {
    if val < 1 {
        None
    } else {
        u16::try_from(val).ok()
    }
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Record { reminder } => try_record(deps, env, reminder),
        HandleMsg::Read { } => try_read(deps, env),
    }
}

fn try_record<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    reminder: String,
) -> StdResult<HandleResponse> {
    let status: String;
    let reminder = reminder.as_bytes();

    // retrieve the config state from storage
    let mut config: State = load(&mut deps.storage, CONFIG_KEY)?;

    if reminder.len() > config.max_size.into() {
        // if reminder content is too long, set status message and do nothing else
        status = String::from("Message is too long. Reminder not recorded.");
    } else {
        // get the canonical address of sender
        let sender_address = deps.api.canonical_address(&env.message.sender)?;

        // create the reminder struct containing content string and timestamp
        let stored_reminder = Reminder {
            content: reminder.to_vec(),
            timestamp: env.block.time
        };

        // save the reminder using a byte vector representation of the sender's address as the key
        save(&mut deps.storage, &sender_address.as_slice().to_vec(), &stored_reminder)?;

        // increment the reminder_count
        config.reminder_count += 1;
        save(&mut deps.storage, CONFIG_KEY, &config)?;

        // set the status message
        status = String::from("Reminder recorded!");
    }

    // Return a HandleResponse with the appropriate status message included in the data field
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Record {
            status,
        })?),
    })
}

fn try_read<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let status: String;
    let mut reminder: Option<String> = None;
    let mut timestamp: Option<u64> = None;

    let sender_address = deps.api.canonical_address(&env.message.sender)?;

    // read the reminder from storage
    let result: Option<Reminder> = may_load(&mut deps.storage, &sender_address.as_slice().to_vec()).ok().unwrap();
    match result {
        // set all response field values
        Some(stored_reminder) => {
            status = String::from("Reminder found.");
            reminder = String::from_utf8(stored_reminder.content).ok();
            timestamp = Some(stored_reminder.timestamp);
        }
        // unless there's an error
        None => { status = String::from("Reminder not found."); }
    };

    // Return a HandleResponse with status message, reminder, and timestamp included in the data field
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Read {
            status,
            reminder,
            timestamp,
        })?),
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Stats { } => query_stats(deps)
    }
}

fn query_stats<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<Binary> {
    // retrieve the config state from storage
    let config: State = load(&deps.storage, CONFIG_KEY)?;
    to_binary(&QueryAnswer::Stats{ reminder_count: config.reminder_count })
}


