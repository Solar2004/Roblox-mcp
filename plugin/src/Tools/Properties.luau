local Util = require(script.Parent.Parent.Util)
local HttpService = game:GetService("HttpService")
local ChangeHistoryService = game:GetService("ChangeHistoryService")

return function(msg)
	if msg.GetInstanceProperties then
		local args = msg.GetInstanceProperties
		local instancePath = args.instancePath
		if not instancePath then
			return HttpService:JSONEncode({ error = "Instance path is required" })
		end

		local instance = Util.getInstanceByPath(instancePath)
		if not instance then
			return HttpService:JSONEncode({ error = "Instance not found: " .. instancePath })
		end

		local properties = {}
		local success, result = pcall(function()
			local basicProps = { "Name", "ClassName", "Parent" }
			for _, prop in ipairs(basicProps) do
				local propSuccess, propValue = pcall(function()
					local val = instance[prop]
					if prop == "Parent" and val then
						return Util.getInstancePath(val)
					elseif val == nil then
						return "nil"
					else
						return tostring(val)
					end
				end)
				if propSuccess then
					properties[prop] = propValue
				end
			end

			local commonProps = {
				"Size", "Position", "Rotation", "CFrame", "Anchored", "CanCollide",
				"Transparency", "BrickColor", "Material", "Color", "Text", "TextColor3",
				"BackgroundColor3", "Image", "ImageColor3", "Visible", "Active", "ZIndex",
				"BorderSizePixel", "BackgroundTransparency", "ImageTransparency",
				"TextTransparency", "Value", "Enabled", "Brightness", "Range",
				"Shadows", "Face", "SurfaceType",
			}

			for _, prop in ipairs(commonProps) do
				local propSuccess, propValue = pcall(function()
					return tostring(instance[prop])
				end)
				if propSuccess then
					properties[prop] = propValue
				end
			end

			if instance:IsA("LuaSourceContainer") then
				properties.Source = instance.Source
				if instance:IsA("BaseScript") then
					properties.Enabled = tostring(instance.Enabled)
				end
			end

			if instance:IsA("Part") then
				properties.Shape = tostring(instance.Shape)
			end

			if instance:IsA("BasePart") then
				properties.TopSurface = tostring(instance.TopSurface)
				properties.BottomSurface = tostring(instance.BottomSurface)
			end

			properties.ChildCount = tostring(#instance:GetChildren())

			return properties
		end)

		if success then
			return HttpService:JSONEncode({
				instancePath = instancePath,
				className = instance.ClassName,
				properties = properties,
			})
		else
			return HttpService:JSONEncode({ error = "Failed to get properties: " .. tostring(result) })
		end
		
	elseif msg.SetProperty then
		local args = msg.SetProperty
		local instancePath = args.instancePath
		local propertyName = args.propertyName
		local propertyValue = args.propertyValue

		if not instancePath or not propertyName then
			return HttpService:JSONEncode({ error = "Instance path and property name are required" })
		end

		local instance = Util.getInstanceByPath(instancePath)
		if not instance then
			return HttpService:JSONEncode({ error = "Instance not found: " .. instancePath })
		end

		local success, result = pcall(function()
			if propertyName == "Parent" or propertyName == "PrimaryPart" then
				if type(propertyValue) == "string" then
					local refInstance = Util.getInstanceByPath(propertyValue)
					if refInstance then
						instance[propertyName] = refInstance
					else
						return { error = propertyName .. " instance not found: " .. propertyValue }
					end
				end
			elseif propertyName == "Name" then
				instance.Name = tostring(propertyValue)
			elseif propertyName == "Source" and instance:IsA("LuaSourceContainer") then
				instance.Source = tostring(propertyValue)
			else
				local convertedValue = Util.convertPropertyValue(instance, propertyName, propertyValue)
				if convertedValue ~= nil then
					instance[propertyName] = convertedValue
				else
					instance[propertyName] = propertyValue
				end
			end

			ChangeHistoryService:SetWaypoint("Set " .. propertyName .. " property")
			return true
		end)

		if success and result ~= false then
			return HttpService:JSONEncode({
				success = true,
				instancePath = instancePath,
				propertyName = propertyName,
				propertyValue = propertyValue,
				message = "Property set successfully",
			})
		else
			return HttpService:JSONEncode({
				error = "Failed to set property: " .. tostring(result),
				instancePath = instancePath,
				propertyName = propertyName,
			})
		end
		
	elseif msg.MassSetProperty then
		local args = msg.MassSetProperty
		local paths = args.paths
		local propertyName = args.propertyName
		local propertyValue = args.propertyValue
		
		local results = {}
		for _, path in ipairs(paths) do
			local inst = Util.getInstanceByPath(path)
			if inst then
				pcall(function() inst[propertyName] = propertyValue end)
				table.insert(results, { path = path, success = true })
			else
				table.insert(results, { path = path, success = false, error = "Not found" })
			end
		end
		ChangeHistoryService:SetWaypoint("Mass Set Property")
		return HttpService:JSONEncode({ results = results })

	elseif msg.MassGetProperty then
		local args = msg.MassGetProperty
		local paths = args.paths
		local propertyName = args.propertyName
		
		local results = {}
		for _, path in ipairs(paths) do
			local inst = Util.getInstanceByPath(path)
			if inst then
				local s, val = pcall(function() return tostring(inst[propertyName]) end)
				if s then
					table.insert(results, { path = path, value = val })
				else
					table.insert(results, { path = path, error = tostring(val) })
				end
			end
		end
		return HttpService:JSONEncode({ results = results })
	end
	
	return nil
end