# SNIP-721 Upgradable Implementation
This is a modification of the SNIP-721 Reference Implementation (including features from SNIP-722 and SNIP-723). Please refer to the [SNIP-721 Reference Implementation](https://github.com/baedrik/snip721-reference-impl) README for the majority of the contract description. I will highlight the differences here.

modified functions (currently these all get the metadata from the first registered provider):
- query_nft_info
- query_private_metadata
- dossier_list 

new stuff:
- RegisterMetadataProvider
- UpdateMetadataProvider
- BatchNftInfo
- BatchPrivateMetadata
- BatchyNftDossier (working title)
- (I decided not to do a BatchyBatchNftDossier because I thought that would just be too much data to be useful)

# Messages
## RegisterMetadataProvider
RegisterMetadataProvider adds a new contract as a metadata provider.  Only the admin address my execute RegisterMetadataProvider.

##### Request
```
{
	"register_metadata_provider": {
		"address": "contract_address_of_metadata_provider",
		"code_hash": "contract_code_hash_of_metadata_provider",
	}
}
```
| Name             | Type                                      | Description                                                                                   | Optional | Value If Omitted     |
|------------------|-------------------------------------------|-----------------------------------------------------------------------------------------------|----------|----------------------|
| address          | string (HumanAddr)                        | Address of the metadata provider contract                                                     | no       |                      |
| code_hash        | string                                    | Code hash of the metadata provider contract                                                   | no       |                      |

##### Response
```
{
	"register_metadata_provider": {
		"status": "success",
	}
}
```
The registered provider address will also be returned in a LogAttribute with the key `registered_provider`.

## UpdateMetadataProvider
UpdateMetadataProvider replaces an existing metadata provider. The previous contract information must be provided and match the stored information. Only the admin address my execute UpdateMetadataProvider.

##### Request
```
{
	"update_metadata_provider": {
		"previous_contract": "contract_address_of_old_metadata_provider"
		"new_contract": "contract_address_of_new_metadata_provider",
		"previous_code_hash": "contract_code_hash_of_old_metadata_provider",
		"new_code_hash": "contract_code_hash_of_new_metadata_provider",
	}
}
```
| Name               | Type                                      | Description                                                                                   | Optional | Value If Omitted     |
|--------------------|-------------------------------------------|-----------------------------------------------------------------------------------------------|----------|----------------------|
| previous_contract  | string (HumanAddr)                        | Address of the metadata provider contract to be replaced                                      | no       |                      |
| new_contract       | string (HumanAddr)                        | Address of the new metadata provider contract                                                 | no       |                      |
| previous_code_hash | string                                    | Code hash of the metadata provider contract to be replaced                                    | no       |                      |
| new_code_hash      | string                                    | Code hash of the new metadata provider contract                                               | no       |                      |

##### Response
```
{
	"update_metadata_provider": {
		"status": "success",
	}
}
```
The updated provider address will also be returned in a LogAttribute with the key `registered_provider`.

# Queries

## BatchNftInfo
BatchNftInfo returns the public metadata of a token **from all registered metadata provider contracts. An optional list of provider addresses can be included to query specific metadata providers.** It follows CW-721 specification, which is based on ERC-721 Metadata JSON Schema.  At most, one of the fields `token_uri` OR `extension` will be defined.

##### Request
```
{
	"batch_nft_info": {
		"token_id": "ID_of_the_token_being_queried",
		"provider_list": [
			"Optional_metadata_provider_contract_address",
			"optional_metadata_provider_contract_address",
			"..."
		]
	}
}
```
| Name           | Type                        | Description                              | Optional | Value if Omitted
|----------------|-----------------------------|------------------------------------------|----------|------------------
| token_id       | string                      | ID of the token being queried            | no       |
| provider_list  | array of string (HumanAddr) | List of addresses of provider contracts  | yes      | nothing

##### Response
```
{
  "batch_nft_info": {
    "metadata": [
      {
        "token_uri": "optional_uri_pointing_to_off-chain_JSON_metadata",
        "extension": {
          "...": "...",
        }
      },
      {
        "token_uri": "optional_uri_pointing_to_off-chain_JSON_metadata",
        "extension": {
          "...": "...",
        }
      },
	  {
		"...": "..."
	  }
    ]
  }
}
```

## BatchPrivateMetadata
BatchPrivateMetadata returns the private metadata of a token (if the querier is permitted to view it) **from all registered metadata provider contracts. An optional list of provider addresses can be included to query specific metadata providers.**  It follows CW-721 metadata specification, which is based on ERC-721 Metadata JSON Schema.  At most, one of the fields `token_uri` OR `extension` will be defined.  If the metadata is [sealed](#enablesealed), no one is permitted to view it until it has been unwrapped with [Reveal](#reveal).  If no [viewer](#viewerinfo) is provided, PrivateMetadata will only display the private metadata if the private metadata is public for this token.

##### Request
```
{
	"batch_private_metadata": {
		"token_id": "ID_of_the_token_being_queried",
		"viewer": {
			"address": "address_of_the_querier_if_supplying_optional_ViewerInfo",
			"viewing_key": "viewer's_key_if_supplying_optional_ViewerInfo"
		}
		"provider_list": [
			"optional_metadata_provider_contract_address",
			"optional_metadata_provider_contract_address",
			"..."
		]
	}
}
```
| Name           | Type                                  | Description                                       | Optional | Value if Omitted
|----------------|---------------------------------------|---------------------------------------------------|----------|-----------------
| token_id       | string                                | ID of the token being queried                     | no       |
| viewer         | ViewerInfo                            | The address and viewing key performing this query | yes      | nothing
| provider_list  | array of string (HumanAddr)           | List of addresses of provider contracts           | yes      | nothing

##### Response
```
{
  "batch_private_metadata": {
    "metadata": [
      {
        "token_uri": "optional_uri_pointing_to_off-chain_JSON_metadata",
        "extension": {
          "...": "...",
        }
      },
      {
        "token_uri": "optional_uri_pointing_to_off-chain_JSON_metadata",
        "extension": {
          "...": "...",
        }
      },
	  {
		"...": "..."
	  }
    ]
  }
}
```

## BatchyNftDossier (working title)
BatchyNftDossier returns all the information about a token that the viewer is permitted to view **from all registered metadata provider contracts. An optional list of provider addresses can be included to query specific metadata providers.**  If no `viewer` is provided, BatchyNftDossier will only display the information that has been made public.  The response may include the owner, the public metadata, the private metadata, the reason the private metadata is not viewable, the royalty information, the mint run information, whether ownership is public, whether the private metadata is public, and (if the querier is the owner,) the approvals for this token as well as the inventory-wide approvals for the owner.  This implementation will only display a token's royalty recipient addresses if the querier has permission to transfer the token.

##### Request
```
{
	"batchy_nft_dossier": {
		"token_id": "ID_of_the_token_being_queried",
		"viewer": {
			"address": "address_of_the_querier_if_supplying_optional_ViewerInfo",
			"viewing_key": "viewer's_key_if_supplying_optional_ViewerInfo"
		},
		"include_expired": true | false,
		"provider_list": [
			"optional_metadata_provider_contract_address",
			"optional_metadata_provider_contract_address",
			"..."
		]
	}
}
```
| Name            | Type                                  | Description                                                           | Optional | Value If Omitted |
|-----------------|---------------------------------------|-----------------------------------------------------------------------|----------|------------------|
| token_id        | string                                | ID of the token being queried                                         | no       |                  |
| viewer          | [ViewerInfo (see above)](#viewerinfo) | The address and viewing key performing this query                     | yes      | nothing          |
| include_expired | bool                                  | True if expired approvals should be included in the response          | yes      | false            |
| provider_list   | array of string (HumanAddr)           | List of addresses of provider contracts                               | yes      | nothing          |

##### Response
```
{
	"batchy_nft_dossier": {
		"owner": "address_of_the_token_owner",
		"public_metadata": {
			"token_uri": "optional_uri_pointing_to_off-chain_JSON_metadata",
			"extension": {
				"...": "..."
			}
		},
		"private_metadata": {
			"token_uri": "optional_uri_pointing_to_off-chain_JSON_metadata",
			"extension": {
				"...": "..."
			}
		},
		"display_private_metadata_error": "optional_error_describing_why_private_metadata_is_not_viewable_if_applicable",
		"royalty_info": {
			"decimal_places_in_rates": 4,
			"royalties": [
				{
					"recipient": "optional_address_that_should_be_paid_this_royalty",
					"rate": 100,
				},
				{
					"...": "..."
				}
			],
		},
		"mint_run_info": {
			"collection_creator": "optional_address_that_instantiated_this_contract",
			"token_creator": "optional_address_that_minted_this_token",
			"time_of_minting": 999999,
			"mint_run": 3,
			"serial_number": 67,
			"quantity_minted_this_run": 1000,
		},
		"owner_is_public": true | false,
		"public_ownership_expiration": "never" | {"at_height": 999999} | {"at_time":999999},
		"private_metadata_is_public": true | false,
		"private_metadata_is_public_expiration": "never" | {"at_height": 999999} | {"at_time":999999},
		"token_approvals": [
			{
				"address": "whitelisted_address",
				"view_owner_expiration": "never" | {"at_height": 999999} | {"at_time":999999},
				"view_private_metadata_expiration": "never" | {"at_height": 999999} | {"at_time":999999},
				"transfer_expiration": "never" | {"at_height": 999999} | {"at_time":999999},
			},
			{
				"...": "..."
			}
		],
		"inventory_approvals": [
			{
				"address": "whitelisted_address",
				"view_owner_expiration": "never" | {"at_height": 999999} | {"at_time":999999},
				"view_private_metadata_expiration": "never" | {"at_height": 999999} | {"at_time":999999},
				"transfer_expiration": "never" | {"at_height": 999999} | {"at_time":999999},
			},
			{
				"...": "..."
			}
		]
	}
}
```
| Name                                  | Type                                                  | Description                                                                            | Optional | 
|---------------------------------------|-------------------------------------------------------|----------------------------------------------------------------------------------------|----------|
| owner                                 | string (HumanAddr)                                    | Address of the token's owner                                                           | yes      |
| public_metadata                       | Metadata                                              | The token's public metadata                                                            | yes      |
| private_metadata                      | Metadata                                              | The token's private metadata                                                           | yes      |
| display_private_metadata_error        | string                                                | If the private metadata is not displayed, the corresponding error message              | yes      |
| royalty_info                          | RoyaltyInfo                                           | The token's RoyaltyInfo                                                                | yes      |
| mint_run_info                         | MintRunInfo                                           | The token's MintRunInfo                                                                | yes      |
| owner_is_public                       | bool                                                  | True if ownership is public for this token                                             | no       |
| public_ownership_expiration           | Expiration                                            | When public ownership expires for this token.  Can be a blockheight, time, or never    | yes      |
| private_metadata_is_public            | bool                                                  | True if private metadata is public for this token                                      | no       |
| private_metadata_is_public_expiration | Expiration                                            | When public display of private metadata expires.  Can be a blockheight, time, or never | yes      |
| token_approvals                       | array of Snip721Approval                              | List of approvals for this token                                                       | yes      |
| inventory_approvals                   | array of Snip721Approval                              | List of inventory-wide approvals for the token's owner                                 | yes      |
