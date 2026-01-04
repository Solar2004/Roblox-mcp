local Util = require(script.Parent.Parent.Util)
local HttpService = game:GetService("HttpService")

return function(msg)
	if msg.SearchFiles then
		local args = msg.SearchFiles
		local query = args.query
		local searchType = args.searchType or "name"

		if not query then
			return HttpService:JSONEncode({ error = "Query is required" })
		end

		local results = {}

		local function searchRecursive(instance)
			local match = false

			if searchType == "name" then
				match = instance.Name:lower():find(query:lower()) ~= nil
			elseif searchType == "type" then
				match = instance.ClassName:lower():find(query:lower()) ~= nil
			elseif searchType == "content" and instance:IsA("LuaSourceContainer") then
				match = instance.Source:lower():find(query:lower()) ~= nil
			end

			if match then
				table.insert(results, {
					name = instance.Name,
					className = instance.ClassName,
					path = Util.getInstancePath(instance),
					hasSource = instance:IsA("LuaSourceContainer"),
				})
			end

			for _, child in ipairs(instance:GetChildren()) do
				searchRecursive(child)
			end
		end

		searchRecursive(game)

		return HttpService:JSONEncode({
			results = results,
			query = query,
			searchType = searchType,
			count = #results,
		})

	elseif msg.SearchObjects then
		local args = msg.SearchObjects
		local query = args.query
		local searchType = args.searchType or "name"
		local propertyName = args.propertyName

		if not query then
			return HttpService:JSONEncode({ error = "Query is required" })
		end

		local results = {}

		local function searchRecursive(instance)
			local match = false

			if searchType == "name" then
				match = instance.Name:lower():find(query:lower()) ~= nil
			elseif searchType == "class" then
				match = instance.ClassName:lower():find(query:lower()) ~= nil
			elseif searchType == "property" and propertyName then
				local success, value = pcall(function()
					return tostring(instance[propertyName])
				end)
				if success then
					match = value:lower():find(query:lower()) ~= nil
				end
			end

			if match then
				table.insert(results, {
					name = instance.Name,
					className = instance.ClassName,
					path = Util.getInstancePath(instance),
				})
			end

			for _, child in ipairs(instance:GetChildren()) do
				searchRecursive(child)
			end
		end

		searchRecursive(game)

		return HttpService:JSONEncode({
			results = results,
			query = query,
			searchType = searchType,
			count = #results,
		})

	elseif msg.SearchByProperty then
		local args = msg.SearchByProperty
		local propertyName = args.propertyName
		local propertyValue = args.propertyValue

		if not propertyName or not propertyValue then
			return HttpService:JSONEncode({ error = "Property name and value are required" })
		end

		local results = {}

		local function searchRecursive(instance)
			local success, value = pcall(function()
				return tostring(instance[propertyName])
			end)

			if success and value:lower():find(propertyValue:lower()) then
				table.insert(results, {
					name = instance.Name,
					className = instance.ClassName,
					path = Util.getInstancePath(instance),
					propertyValue = value,
				})
			end

			for _, child in ipairs(instance:GetChildren()) do
				searchRecursive(child)
			end
		end

		searchRecursive(game)

		return HttpService:JSONEncode({
			propertyName = propertyName,
			propertyValue = propertyValue,
			results = results,
			count = #results,
		})
	end
	return nil
end
