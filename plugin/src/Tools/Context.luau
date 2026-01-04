local Util = require(script.Parent.Parent.Util)
local HttpService = game:GetService("HttpService")

return function(msg)
	if msg.GetPlaceInfo then
		local info = {
			placeName = game.Name,
			placeId = game.PlaceId,
			gameId = game.GameId,
			jobId = game.JobId,
			workspace = {
				name = workspace.Name,
				className = workspace.ClassName,
			},
		}
		return HttpService:JSONEncode(info)

	elseif msg.GetClassInfo then
		local args = msg.GetClassInfo
		local className = args.className
		if not className then return HttpService:JSONEncode({ error = "Class name required" }) end

		local success, tempInstance = pcall(function()
			return Instance.new(className)
		end)

		if not success then
			return HttpService:JSONEncode({ error = "Invalid class name: " .. className })
		end

		local classInfo = {
			className = className,
			properties = {},
			methods = {},
			events = {},
		}

		local commonProps = {
			"Name", "ClassName", "Parent", "Size", "Position", "Rotation", "CFrame",
			"Anchored", "CanCollide", "Transparency", "BrickColor", "Material", "Color",
			"Text", "TextColor3", "BackgroundColor3", "Image", "ImageColor3", "Visible",
			"Active", "ZIndex", "BorderSizePixel", "BackgroundTransparency", "ImageTransparency",
			"TextTransparency", "Value", "Enabled", "Brightness", "Range", "Shadows",
		}

		for _, prop in ipairs(commonProps) do
			local propSuccess, _ = pcall(function() return tempInstance[prop] end)
			if propSuccess then table.insert(classInfo.properties, prop) end
		end

		local commonMethods = {
			"Destroy", "Clone", "FindFirstChild", "FindFirstChildOfClass",
			"GetChildren", "IsA", "IsAncestorOf", "IsDescendantOf", "WaitForChild",
		}

		for _, method in ipairs(commonMethods) do
			local mSuccess, _ = pcall(function() return tempInstance[method] end)
			if mSuccess then table.insert(classInfo.methods, method) end
		end

		tempInstance:Destroy()
		return HttpService:JSONEncode(classInfo)
	end
	return nil
end
