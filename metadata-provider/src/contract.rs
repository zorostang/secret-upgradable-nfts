use cosmwasm_std::{
    debug_print, to_binary, Api, CanonicalAddr, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, InitResponse, InitResult, Querier, QueryResult, ReadonlyStorage, StdError,
    StdResult, Storage,
};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};

use crate::msg::{
    HandleAnswer, HandleMsg, InitMsg, QueryAnswer, QueryMsg, ResponseStatus::Success,
};
use crate::state::{
    load, may_load, save, Config, BLOCK_KEY, CONFIG_KEY, PREFIX_PRIV_META, PREFIX_PUB_META,
    PREFIX_VIEW_KEY,
};
use crate::viewing_key::{ViewingKey, VIEWING_KEY_SIZE};

use secret_toolkit::snip721::{Metadata, ViewerInfo};
use secret_toolkit::utils::{pad_handle_result, pad_query_result};

/// pad handle responses and log attributes to blocks of 256 bytes to prevent leaking info based on
/// response size
pub const BLOCK_SIZE: usize = 256;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    let config = Config {
        name: msg.name,
        symbol: msg.symbol,
        admin: deps.api.canonical_address(&env.message.sender)?,
        token_address: deps.api.canonical_address(&msg.token_address)?,
        token_code_hash: msg.token_code_hash,
    };

    save(&mut deps.storage, CONFIG_KEY, &config)?;

    debug_print!("Contract was initialized by {}", env.message.sender);

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    save(&mut deps.storage, BLOCK_KEY, &env.block)?;
    let mut config: Config = load(&deps.storage, CONFIG_KEY)?;

    let response = match msg {
        HandleMsg::SetMetadata {
            token_id,
            public_metadata,
            private_metadata,
            ..
        } => handle_set_metadata(
            deps,
            env,
            &config,
            &token_id,
            public_metadata,
            private_metadata,
        ),
        HandleMsg::SetViewingKey { key, .. } => handle_set_key(deps, env, &config, key),
        HandleMsg::ChangeAdmin { address, .. } => {
            handle_change_admin(deps, env, &mut config, &address)
        }
    };
    pad_handle_result(response, BLOCK_SIZE)
}

pub fn handle_set_metadata<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    _config: &Config,
    token_id: &str,
    public_metadata: Option<Metadata>,
    private_metadata: Option<Metadata>,
) -> HandleResult {
    if let Some(public) = public_metadata {
        set_metadata_impl(&mut deps.storage, token_id, PREFIX_PUB_META, &public)?;
    }
    if let Some(private) = private_metadata {
        set_metadata_impl(&mut deps.storage, token_id, PREFIX_PRIV_META, &private)?;
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::SetMetadata { status: Success })?),
    })
}

/// Returns StdResult<()>
///
/// sets new metadata
///
/// # Arguments
///
/// * `storage` - a mutable reference to the contract's storage
/// * `token` - a reference to the token whose metadata should be updated
/// * `idx` - the token identifier index
/// * `prefix` - storage prefix for the type of metadata being updated
/// * `metadata` - a reference to the new metadata
#[allow(clippy::too_many_arguments)]
fn set_metadata_impl<S: Storage>(
    storage: &mut S,
    token_id: &str,
    prefix: &[u8],
    metadata: &Metadata,
) -> StdResult<()> {
    enforce_metadata_field_exclusion(metadata)?;
    let mut meta_store = PrefixedStorage::new(prefix, storage);
    save(&mut meta_store, &token_id.as_bytes(), metadata)?;
    Ok(())
}

/// Returns StdResult<()>
///
/// makes sure that Metadata does not have both `token_uri` and `extension`
///
/// # Arguments
///
/// * `metadata` - a reference to Metadata
fn enforce_metadata_field_exclusion(metadata: &Metadata) -> StdResult<()> {
    if metadata.token_uri.is_some() && metadata.extension.is_some() {
        return Err(StdError::generic_err(
            "Metadata can not have BOTH token_uri AND extension",
        ));
    }
    Ok(())
}

pub fn handle_set_key<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _config: &Config,
    key: String,
) -> HandleResult {
    let vk = ViewingKey(key.clone());
    let message_sender = deps.api.canonical_address(&env.message.sender)?;
    let mut key_store = PrefixedStorage::new(PREFIX_VIEW_KEY, &mut deps.storage);
    save(&mut key_store, message_sender.as_slice(), &vk.to_hashed())?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::ViewingKey { key })?),
    })
}

pub fn handle_change_admin<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    config: &mut Config,
    address: &HumanAddr,
) -> HandleResult {
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;
    if config.admin != sender_raw {
        return Err(StdError::generic_err(
            "This is an admin command and can only be run from the admin address",
        ));
    }
    let new_admin = deps.api.canonical_address(address)?;
    if new_admin != config.admin {
        config.admin = new_admin;
        save(&mut deps.storage, CONFIG_KEY, &config)?;
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::ChangeAdmin { status: Success })?),
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    let response = match msg {
        QueryMsg::NftInfo { token_id, viewer } => query_nft_info(deps, &token_id, viewer),
        QueryMsg::PrivateMetadata { token_id, viewer } => {
            query_private_metadata(deps, &token_id, viewer)
        }
    };
    pad_query_result(response, BLOCK_SIZE)
}

fn query_nft_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    token_id: &str,
    viewer: Option<ViewerInfo>,
) -> QueryResult {
    // check viewing key
    let _viewer_raw = get_querier(deps, viewer)?;

    // load public metadata
    let meta_store = ReadonlyPrefixedStorage::new(PREFIX_PUB_META, &deps.storage);
    let meta: Metadata = may_load(&meta_store, &token_id.as_bytes())?.unwrap_or_default();

    to_binary(&QueryAnswer::NftInfo {
        token_uri: meta.token_uri,
        extension: meta.extension,
    })
}

fn query_private_metadata<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    token_id: &str,
    viewer: Option<ViewerInfo>,
) -> QueryResult {
    // check viewing key
    let _viewer_raw = get_querier(deps, viewer)?;

    // load private metadata
    let meta_store = ReadonlyPrefixedStorage::new(PREFIX_PRIV_META, &deps.storage);
    let meta: Metadata = may_load(&meta_store, &token_id.as_bytes())?.unwrap_or_default();

    to_binary(&QueryAnswer::PrivateMetadata {
        token_uri: meta.token_uri,
        extension: meta.extension,
    })
}

/// Returns StdResult<bool> result of validating an address' viewing key
///
/// # Arguments
///
/// * `storage` - a reference to the contract's storage
/// * `address` - a reference to the address whose key should be validated
/// * `viewing_key` - String key used for authentication
fn check_key<S: ReadonlyStorage>(
    storage: &S,
    address: &CanonicalAddr,
    viewing_key: String,
) -> StdResult<()> {
    // load the address' key
    let read_key = ReadonlyPrefixedStorage::new(PREFIX_VIEW_KEY, storage);
    let load_key: [u8; VIEWING_KEY_SIZE] =
        may_load(&read_key, address.as_slice())?.unwrap_or([0u8; VIEWING_KEY_SIZE]);
    let input_key = ViewingKey(viewing_key);
    // if key matches
    if input_key.check_viewing_key(&load_key) {
        return Ok(());
    }
    Err(StdError::generic_err(
        "Wrong viewing key for this address or viewing key not set",
    ))
}

/// Returns StdResult<Option<CanonicalAddr>> from determining the querying address (if possible) either
/// from a permit validation or a ViewerInfo
///
/// # Arguments
///
/// * `deps` - a reference to Extern containing all the contract's external dependencies
/// * `viewer` - optional address and key making an authenticated query request
/// * `from_permit` - the address derived from an Owner permit, if applicable
fn get_querier<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    viewer: Option<ViewerInfo>,
) -> StdResult<Option<CanonicalAddr>> {
    let viewer_raw = viewer
        .map(|v| {
            let raw = deps.api.canonical_address(&v.address)?;
            check_key(&deps.storage, &raw, v.viewing_key)?;
            Ok(raw)
        })
        .transpose()?;
    Ok(viewer_raw)
}
