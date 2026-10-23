#include "PathImage.h"

#include "NxFiles.h"

#include <utility>

#include <nlnx/nx.hpp>

namespace ms
{
	Texture path_image(const std::string &path)
	{
		size_t slash = path.find('/');

		if (slash == std::string::npos)
			return Texture();

		const std::pair<std::string, nl::node> files[] = {
			{ "UI", NxFiles::ui_classic }, { "Map", nl::nx::map }, { "Effect", nl::nx::effect },
			{ "Item", nl::nx::item }, { "Skill", nl::nx::skill }, { "Mob", nl::nx::mob }, { "Npc", nl::nx::npc },
			{ "Character", nl::nx::character }, { "Etc", nl::nx::etc }, { "Quest", nl::nx::quest },
			{ "Reactor", nl::nx::reactor }
		};

		std::string file = path.substr(0, slash);
		std::string rest = path.substr(slash + 1);

		for (const auto &[name, root] : files)
		{
			if (name != file)
				continue;

			nl::node found = root.resolve(rest);

			if (found.data_type() != nl::node::type::bitmap && name == "UI")
			{
				found = nl::nx::ui.resolve(rest);

				if (found.data_type() != nl::node::type::bitmap && rest.rfind("UIWindow.img/", 0) == 0)
					found = nl::nx::ui.resolve("UIWindow2.img/" + rest.substr(13));
			}

			return Texture(found);
		}

		return Texture();
	}
}
