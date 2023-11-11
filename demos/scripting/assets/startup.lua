local Transform = s"Transform"
local Sprite = s"Sprite"

local meta = world.assets.root
local components = world.components
local entities = world.resources:get(s"Entities")

local ent = entities:create()
components:insert(ent, Transform:create())
local sprite = Sprite:create()
sprite.image = meta.sprite
components:insert(ent, sprite)

