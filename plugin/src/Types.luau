export type SmartUnpackArgs = {
	targetName: string,
}

export type InsertModelArgs = {
	query: string?,
	assetId: number?,
}

export type RunCommandArgs = {
	command: string,
}

export type GetInstancePropertiesArgs = {
	instancePath: string,
}

export type SetPropertyArgs = {
	instancePath: string,
	propertyName: string,
	propertyValue: any,
}

export type GetFileTreeArgs = {
	path: string?,
}

export type GetServicesArgs = {
	serviceName: string?,
}

export type GetProjectStructureArgs = {
	path: string?,
	maxDepth: number?,
	scriptsOnly: boolean?,
}

export type GetInstanceChildrenArgs = {
	instancePath: string,
}

export type SearchFilesArgs = {
	query: string,
	searchType: string?,
}

export type SearchObjectsArgs = {
	query: string,
	searchType: string?,
	propertyName: string?,
}

export type SearchByPropertyArgs = {
	propertyName: string,
	propertyValue: string,
}

export type CreateObjectArgs = {
	className: string,
	parent: string,
	name: string?,
	properties: { [string]: any }?,
}

export type DeleteObjectArgs = {
	instancePath: string,
}

export type MassSetPropertyArgs = {
	paths: {string},
	propertyName: string,
	propertyValue: any,
}

export type MassGetPropertyArgs = {
	paths: {string},
	propertyName: string,
}

export type SetCalculatedPropertyArgs = {
	paths: {string},
	propertyName: string,
	formula: string,
	variables: {[string]: number}?,
}

export type SetRelativePropertyArgs = {
	paths: {string},
	propertyName: string,
	operation: string,
	value: any,
	component: string?,
}

export type GetPlaceInfoArgs = {
}

export type GetClassInfoArgs = {
	className: string,
}

export type InsertAssetsArgs = {
	asset_ids: {number},
}

export type ToolArgs = { InsertModel: InsertModelArgs } 
	| { RunCommand: RunCommandArgs } 
	| { GetInstanceProperties: GetInstancePropertiesArgs } 
	| { SetProperty: SetPropertyArgs }
	| { GetFileTree: GetFileTreeArgs }
	| { GetServices: GetServicesArgs }
	| { GetProjectStructure: GetProjectStructureArgs }
	| { GetInstanceChildren: GetInstanceChildrenArgs }
	| { SearchFiles: SearchFilesArgs }
	| { SearchObjects: SearchObjectsArgs }
	| { SearchByProperty: SearchByPropertyArgs }
	| { CreateObject: CreateObjectArgs }
	| { DeleteObject: DeleteObjectArgs }
	| { MassSetProperty: MassSetPropertyArgs }
	| { MassGetProperty: MassGetPropertyArgs }
	| { SetCalculatedProperty: SetCalculatedPropertyArgs }
	| { SetRelativeProperty: SetRelativePropertyArgs }
	| { GetPlaceInfo: GetPlaceInfoArgs }
	| { GetClassInfo: GetClassInfoArgs }
	| { InsertAssets: InsertAssetsArgs }
	| { SmartUnpack: SmartUnpackArgs }

export type ToolFunction = (ToolArgs) -> string?

return {}
