local Util = require(script.Parent.Parent.Util)
local HttpService = game:GetService("HttpService")
local ChangeHistoryService = game:GetService("ChangeHistoryService")

return function(msg)
	if msg.CreateScript then
		local args = msg.CreateScript
		local name = args.name
		local parentPath = args.parent
		local scriptType = args.script_type or "Script"
		local source = args.source or ""

		if not name or not parentPath then
			return HttpService:JSONEncode({ error = "Name and parent are required" })
		end

		local parent = Util.getInstanceByPath(parentPath)
		if not parent then
			return HttpService:JSONEncode({ error = "Parent not found: " .. parentPath })
		end

		-- Validate script type
		if scriptType ~= "Script" and scriptType ~= "LocalScript" and scriptType ~= "ModuleScript" then
			return HttpService:JSONEncode({ error = "Invalid script type: " .. tostring(scriptType) })
		end

		local success, newScript = pcall(function()
			local s = Instance.new(scriptType)
			s.Name = name
			s.Source = source
			s.Parent = parent
			return s
		end)

		if success and newScript then
			ChangeHistoryService:SetWaypoint("Create Script " .. name)
			return HttpService:JSONEncode({
				success = true,
				instancePath = Util.getInstancePath(newScript),
				message = "Created " .. scriptType
			})
		else
			return HttpService:JSONEncode({ error = "Failed to create script: " .. tostring(newScript) })
		end

	elseif msg.UpdateScript then
		local args = msg.UpdateScript
		local instancePath = args.instancePath
		local source = args.source

		if not instancePath or not source then
			return HttpService:JSONEncode({ error = "Instance path and source are required" })
		end

		local instance = Util.getInstanceByPath(instancePath)
		if not instance then
			return HttpService:JSONEncode({ error = "Instance not found" })
		end

		if not instance:IsA("LuaSourceContainer") then
			return HttpService:JSONEncode({ error = "Instance is not a script" })
		end

		local success, err = pcall(function()
			instance.Source = source
		end)

		if success then
			ChangeHistoryService:SetWaypoint("Update Script Source")
			return HttpService:JSONEncode({ success = true, message = "Script updated" })
		else
			return HttpService:JSONEncode({ error = "Failed to update script: " .. tostring(err) })
		end

	elseif msg.ReadScript then
		local args = msg.ReadScript
		local instancePath = args.instancePath

		if not instancePath then
			return HttpService:JSONEncode({ error = "Instance path is required" })
		end

		local instance = Util.getInstanceByPath(instancePath)
		if not instance then
			return HttpService:JSONEncode({ error = "Instance not found" })
		end

		if not instance:IsA("LuaSourceContainer") then
			return HttpService:JSONEncode({ error = "Instance is not a script" })
		end

		return HttpService:JSONEncode({
			success = true,
			source = instance.Source,
			instancePath = instancePath
		})
	end

	return nil
end
