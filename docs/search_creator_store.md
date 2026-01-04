# Search Creator Store Tool

## Overview
The `search_creator_store` tool allows you to search and download assets from the Roblox Creator Store (formerly known as Toolbox). This tool uses the official Roblox API to find models, scripts, audio, and other assets.

## Features
- ✅ Search for assets by query
- ✅ Filter by asset type (Model, Audio, Decal, Plugin, MeshPart, Video, FontFamily)
- ✅ Limit results (1-100)
- ✅ Optional automatic download to Desktop/RobloxAssets folder
- ✅ Returns asset details (ID, name, creator, description)

## Usage

### Basic Search
```typescript
// Search for models
await search_creator_store({
    query: "sword",
    asset_type: "Model",
    limit: 10
});
```

### Search and Download
```typescript
// Search and download audio files
await search_creator_store({
    query: "background music",
    asset_type: "Audio",
    limit: 5,
    download: true
});
```

## Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | string | ✅ Yes | - | Search query for assets |
| `asset_type` | string | ❌ No | "Model" | Asset type: Audio, Model, Decal, Plugin, MeshPart, Video, FontFamily |
| `limit` | number | ❌ No | 10 | Maximum number of results (1-100) |
| `download` | boolean | ❌ No | false | Download found assets to Desktop folder |

## Asset Types

- **Model**: 3D models, buildings, vehicles, characters, etc.
- **Audio**: Music, sound effects, ambient sounds
- **Decal**: Images, textures, logos
- **Plugin**: Roblox Studio plugins
- **MeshPart**: Custom mesh parts
- **Video**: Video files
- **FontFamily**: Custom fonts

## Response Format

The tool returns a formatted text response with:
- Number of assets found
- For each asset:
  - Name and Asset ID
  - Creator name
  - Description (truncated to 100 characters)
- If download is enabled:
  - List of downloaded files with full paths

## Example Responses

### Search Only
```
Found 5 Model assets:

1. Medieval Sword (ID: 123456789)
   Creator: SwordMaster
   Description: A detailed medieval sword with realistic textures...

2. Katana (ID: 987654321)
   Creator: WeaponSmith
   Description: Traditional Japanese katana with custom animations...
```

### Search and Download
```
Found 3 Audio assets:

1. Epic Battle Music (ID: 111222333)
   Creator: MusicComposer
   Description: Intense orchestral music perfect for combat scenes...

2. Ambient Forest (ID: 444555666)
   Creator: SoundDesigner
   Description: Peaceful forest ambience with birds chirping...

✓ Downloaded 3 assets to Desktop/RobloxAssets/:
  - C:\Users\YourName\Desktop\RobloxAssets\Epic_Battle_Music_111222333.rbxm
  - C:\Users\YourName\Desktop\RobloxAssets\Ambient_Forest_444555666.rbxm
```

## API Endpoint

This tool uses the official Roblox Creator Store API:
```
GET https://apis.roblox.com/toolbox-service/v2/assets:search
```

## Notes

- Downloaded assets are saved as `.rbxm` files (Roblox Model format)
- Files are automatically organized in `Desktop/RobloxAssets/` folder
- Asset names are sanitized (spaces and slashes replaced with underscores)
- The tool handles API rate limits gracefully
- No authentication required for searching public assets

## Error Handling

The tool provides clear error messages for:
- Failed API requests
- Invalid asset types
- Download failures
- File system errors

## Integration with AI Agent

This tool is designed to work seamlessly with AI agents for:
- Finding specific assets based on natural language descriptions
- Bulk downloading asset collections
- Building asset libraries for projects
- Discovering new content from verified creators

## Future Enhancements

Potential improvements:
- Filter by creator (user/group)
- Sort by relevance, trending, or top rated
- Price filtering (free vs paid)
- Pagination support for large result sets
- Direct insertion into Roblox Studio
