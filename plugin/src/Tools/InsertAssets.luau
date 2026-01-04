local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local INSERT_MAX_SEARCH_DEPTH = 2048
local INSERT_MAX_DISTANCE_AWAY = 20

local function getInsertPosition()
	local camera = workspace.CurrentCamera
	local viewportPoint = camera.ViewportSize / 2
	local unitRay = camera:ViewportPointToRay(viewportPoint.X, viewportPoint.Y, 0)

	local ray = Ray.new(unitRay.Origin, unitRay.Direction * INSERT_MAX_SEARCH_DEPTH)
	local params = RaycastParams.new()
	params.BruteForceAllSlow = true

	local result = workspace:Raycast(ray.Origin, ray.Direction, params)

	if result then
		return result.Position
	else
		return camera.CFrame.Position + unitRay.Direction * INSERT_MAX_DISTANCE_AWAY
	end
end

local function collapseObjectsIntoContainer(objects: { Instance }): Instance?
	local isPhysical = false
	for _, object in objects do
		if object:IsA("PVInstance") then
			isPhysical = true
			break
		end
	end

	if isPhysical then
		local model = Instance.new("Model")
		for _, object in objects do
			object.Parent = model
		end
		return model
	end

	if #objects > 1 then
		local folder = Instance.new("Folder")
		for _, object in objects do
			object.Parent = folder
		end
		return folder
	end

	return objects[1]
end

-- Smart script placement: moves scripts to appropriate locations
local function organizeScripts(instance: Instance, results: {any})
	local ServerScriptService = game:GetService("ServerScriptService")
	local StarterPlayer = game:GetService("StarterPlayer")
	local ReplicatedStorage = game:GetService("ReplicatedStorage")
	
	-- Find all scripts recursively
	local function findScripts(parent: Instance)
		for _, child in parent:GetChildren() do
			if child:IsA("Script") and not child:IsA("LocalScript") and not child:IsA("ModuleScript") then
				-- Move server scripts to ServerScriptService
				local newParent = ServerScriptService:FindFirstChild(instance.Name)
				if not newParent then
					newParent = Instance.new("Folder")
					newParent.Name = instance.Name
					newParent.Parent = ServerScriptService
				end
				child.Parent = newParent
				table.insert(results.scripts_moved, {
					name = child.Name,
					type = "Script",
					location = "ServerScriptService/" .. instance.Name
				})
			elseif child:IsA("LocalScript") then
				-- Move LocalScripts to StarterPlayerScripts
				local newParent = StarterPlayer.StarterPlayerScripts:FindFirstChild(instance.Name)
				if not newParent then
					newParent = Instance.new("Folder")
					newParent.Name = instance.Name
					newParent.Parent = StarterPlayer.StarterPlayerScripts
				end
				child.Parent = newParent
				table.insert(results.scripts_moved, {
					name = child.Name,
					type = "LocalScript",
					location = "StarterPlayerScripts/" .. instance.Name
				})
			elseif child:IsA("ModuleScript") then
				-- Move ModuleScripts to ReplicatedStorage
				local newParent = ReplicatedStorage:FindFirstChild(instance.Name)
				if not newParent then
					newParent = Instance.new("Folder")
					newParent.Name = instance.Name
					newParent.Parent = ReplicatedStorage
				end
				child.Parent = newParent
				table.insert(results.scripts_moved, {
					name = child.Name,
					type = "ModuleScript",
					location = "ReplicatedStorage/" .. instance.Name
				})
			else
				-- Recursively check children
				findScripts(child)
			end
		end
	end
	
	findScripts(instance)
end

local function loadAsset(assetId: number): Instance?
	local success, objects = pcall(function()
		return game:GetObjects("rbxassetid://"..assetId)
	end)
	
	if not success or not objects then
		return nil
	end
	
	return collapseObjectsIntoContainer(objects)
end

local function handleInsertAssets(args: Types.ToolArgs): string?
	if not args["InsertAssets"] then
		return nil
	end

	local insertAssetsArgs = args["InsertAssets"]
	local assetIds = insertAssetsArgs.asset_ids
	
	if not assetIds or #assetIds == 0 then
		error("InsertAssets requires at least one asset_id")
	end
	
	local inserted = {}
	local failed = {}
	local position = getInsertPosition()
	local offset = 0
	
	for _, assetId in assetIds do
		local instance = loadAsset(assetId)
		if instance then
			local name = instance.Name
			local i = 1
			while workspace:FindFirstChild(name) do
				name = instance.Name .. i
				i += 1
			end
			instance.Name = name
			
			-- Track script movements
			local scriptResults = {scripts_moved = {}}
			
			-- Organize scripts before parenting to workspace
			organizeScripts(instance, scriptResults)
			
			-- Parent to workspace
			instance.Parent = workspace
			
			if instance:IsA("Model") then
				-- Offset each model so they don't overlap
				instance:PivotTo(CFrame.new(position + Vector3.new(offset, 0, 0)))
				offset += 5
			end
			
			table.insert(inserted, {
				id = assetId,
				name = name,
				scripts = scriptResults.scripts_moved
			})
		else
			table.insert(failed, assetId)
		end
	end
	
	local result = "Inserted " .. #inserted .. " assets into Workspace"
	if #inserted > 0 then
		result = result .. ":\n"
		for _, info in inserted do
			result = result .. "  - " .. info.name .. " (ID: " .. info.id .. ")\n"
			if #info.scripts > 0 then
				result = result .. "    📜 Scripts organized:\n"
				for _, script in info.scripts do
					result = result .. "      • " .. script.name .. " (" .. script.type .. ") → " .. script.location .. "\n"
				end
			end
		end
	end
	
	if #failed > 0 then
		result = result .. "\nFailed to insert " .. #failed .. " assets:\n"
		for _, id in failed do
			result = result .. "  - Asset ID: " .. id .. "\n"
		end
	end
	
	return result
end

return handleInsertAssets :: Types.ToolFunction
