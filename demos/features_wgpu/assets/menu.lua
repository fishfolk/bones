local menuData = world.resources:get(schema("MenuData"))

-- Increment the frame counter
menuData.frame = menuData.frame + 1

if menuData.frame % 30 == 0 then 
  info(menuData)
end

