local Util = require(script.Parent.Parent.Util)
local HttpService = game:GetService("HttpService")

local function buildFileTree(instance, depth)
	if depth > 10 then
		return { name = instance.Name, className = instance.ClassName, children = {} }
	end

	local node = {
		name = instance.Name,
		className = instance.ClassName,
		path = Util.getInstancePath(instance),
		children = {},
	}

	if instance:IsA("LuaSourceContainer") then
		node.hasSource = true
		node.scriptType = instance.ClassName
	end

	for _, child in ipairs(instance:GetChildren()) do
		table.insert(node.children, buildFileTree(child, depth + 1))
	end

	return node
end

local function getStructure(instance, depth, currentPath, maxDepth, showScriptsOnly)
	if depth > maxDepth then
		return {
			name = instance.Name,
			className = instance.ClassName,
			path = Util.getInstancePath(instance),
			childCount = #instance:GetChildren(),
			hasMore = true,
			note = "Max depth reached - use this path to explore further",
		}
	end

	local node = {
		name = instance.Name,
		className = instance.ClassName,
		path = Util.getInstancePath(instance),
		children = {},
	}

	if instance:IsA("LuaSourceContainer") then
		node.hasSource = true
		node.scriptType = instance.ClassName
		if instance:IsA("BaseScript") then
			node.enabled = instance.Enabled
		end
	end

	if instance:IsA("GuiObject") then
		node.visible = instance.Visible
		if instance:IsA("Frame") or instance:IsA("ScreenGui") then
			node.guiType = "container"
		elseif instance:IsA("TextLabel") or instance:IsA("TextButton") then
			node.guiType = "text"
			if instance.Text and instance.Text ~= "" then
				node.text = instance.Text
			end
		elseif instance:IsA("ImageLabel") or instance:IsA("ImageButton") then
			node.guiType = "image"
		end
	end

	local children = instance:GetChildren()
	if showScriptsOnly then
		local scriptChildren = {}
		for _, child in ipairs(children) do
			if child:IsA("BaseScript") or child:IsA("Folder") or child:IsA("ModuleScript") then
				table.insert(scriptChildren, child)
			end
		end
		children = scriptChildren
	end

	local childCount = #children
	if childCount > 20 and depth < maxDepth then
		local classGroups = {}
		for _, child in ipairs(children) do
			local className = child.ClassName
			if not classGroups[className] then
				classGroups[className] = {}
			end
			table.insert(classGroups[className], child)
		end

		node.childSummary = {}
		for className, classChildren in pairs(classGroups) do
			table.insert(node.childSummary, {
				className = className,
				count = #classChildren,
				examples = {
					classChildren[1] and classChildren[1].Name,
					classChildren[2] and classChildren[2].Name,
				},
			})
		end

		for className, classChildren in pairs(classGroups) do
			for i = 1, math.min(3, #classChildren) do
				table.insert(node.children, getStructure(classChildren[i], depth + 1, currentPath, maxDepth, showScriptsOnly))
			end
			if #classChildren > 3 then
				table.insert(node.children, {
					name = "... " .. (#classChildren - 3) .. " more " .. className .. " objects",
					className = "MoreIndicator",
					path = Util.getInstancePath(instance) .. " [" .. className .. " children]",
					note = "Use specific path to explore these objects",
				})
			end
		end
	else
		for _, child in ipairs(children) do
			table.insert(node.children, getStructure(child, depth + 1, currentPath, maxDepth, showScriptsOnly))
		end
	end

	return node
end

return function(msg)
	if msg.GetFileTree then
		local args = msg.GetFileTree
		local path = args.path or ""
		local startInstance = Util.getInstanceByPath(path)

		if not startInstance then
			return HttpService:JSONEncode({ error = "Path not found: " .. path })
		end

		return HttpService:JSONEncode({
			tree = buildFileTree(startInstance, 0),
			timestamp = tick(),
		})

	elseif msg.GetServices then
		local args = msg.GetServices
		local serviceName = args.serviceName

		if serviceName then
			local service = Util.safeCall(game.GetService, game, serviceName)
			if service then
				return HttpService:JSONEncode({
					service = {
						name = service.Name,
						className = service.ClassName,
						path = Util.getInstancePath(service),
						childCount = #service:GetChildren(),
					},
				})
			else
				return HttpService:JSONEncode({ error = "Service not found: " .. serviceName })
			end
		else
			local services = {}
			local commonServices = {
				"Workspace", "Players", "StarterGui", "StarterPack", "StarterPlayer",
				"ReplicatedStorage", "ServerStorage", "ServerScriptService",
				"HttpService", "TeleportService", "DataStoreService",
			}

			for _, sName in ipairs(commonServices) do
				local service = Util.safeCall(game.GetService, game, sName)
				if service then
					table.insert(services, {
						name = service.Name,
						className = service.ClassName,
						path = Util.getInstancePath(service),
						childCount = #service:GetChildren(),
					})
				end
			end

			return HttpService:JSONEncode({ services = services })
		end

	elseif msg.GetProjectStructure then
		local args = msg.GetProjectStructure
		local startPath = args.path or ""
		local maxDepth = args.maxDepth or 3
		local showScriptsOnly = args.scriptsOnly or false

		local startInstance
		if startPath == "" or startPath == "game" then
			local services = {}
			local mainServices = {
				"Workspace", "ServerScriptService", "ServerStorage",
				"ReplicatedStorage", "StarterGui", "StarterPack",
				"StarterPlayer", "Players",
			}

			for _, sName in ipairs(mainServices) do
				local service = Util.safeCall(game.GetService, game, sName)
				if service then
					local serviceInfo = {
						name = service.Name,
						className = service.ClassName,
						path = Util.getInstancePath(service),
						childCount = #service:GetChildren(),
						hasChildren = #service:GetChildren() > 0,
					}
					table.insert(services, serviceInfo)
				end
			end

			return HttpService:JSONEncode({
				type = "service_overview",
				services = services,
				timestamp = tick(),
				note = "Use path parameter to explore specific locations (e.g., 'game.ServerScriptService')",
			})
		else
			startInstance = Util.getInstanceByPath(startPath)
			if not startInstance then
				return HttpService:JSONEncode({ error = "Path not found: " .. startPath })
			end
		end

		local result = getStructure(startInstance, 0, startPath, maxDepth, showScriptsOnly)
		result.requestedPath = startPath
		result.maxDepth = maxDepth
		result.scriptsOnly = showScriptsOnly
		result.timestamp = tick()

		return HttpService:JSONEncode(result)

	elseif msg.GetInstanceChildren then
		local args = msg.GetInstanceChildren
		local instancePath = args.instancePath
		if not instancePath then
			return HttpService:JSONEncode({ error = "Instance path is required" })
		end

		local instance = Util.getInstanceByPath(instancePath)
		if not instance then
			return HttpService:JSONEncode({ error = "Instance not found: " .. instancePath })
		end

		local children = {}
		for _, child in ipairs(instance:GetChildren()) do
			table.insert(children, {
				name = child.Name,
				className = child.ClassName,
				path = Util.getInstancePath(child),
				hasChildren = #child:GetChildren() > 0,
				hasSource = child:IsA("LuaSourceContainer"),
			})
		end

		return HttpService:JSONEncode({
			instancePath = instancePath,
			children = children,
			count = #children,
		})
	end
	return nil
end
