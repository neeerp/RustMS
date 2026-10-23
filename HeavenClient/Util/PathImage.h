#pragma once

#include "../Graphics/Texture.h"

#include <string>

namespace ms
{
	// An image by its path in the game data, such as UI/UIWindow.img/QuestIcon/4/0, or an empty texture when the path
	// names no image. UI paths name the v83 UI; our UI.nx has the quest icons under UIWindow2.img.
	Texture path_image(const std::string &path);
}
