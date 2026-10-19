#pragma once

#include <algorithm>
#include <cstdint>
#include <cstdlib>
#include <string>
#include <vector>

namespace ms
{
	// The choices of an NPC's slide menu (dialogue type 14; v83 FUN_0076b6d2 and FUN_0076b5c8).
	namespace slide_menu
	{
		struct Entry
		{
			int32_t id;
			std::string label;
		};

		struct Menu
		{
			// Smallest id first.
			std::vector<Entry> entries;
			// The entry in the middle at first: the second when there are two or more.
			size_t start;
			// Nothing to choose: there are no icons and Move stays disabled.
			bool empty;
		};

		// v83 reads `#<id>#<label>` pairs from the text and a "#" it adds to the end, so the last label needs
		// no "#" of its own. Where v83 would loop forever (text not starting with "#") or read past the end
		// (text ending with "#"), this stops.
		inline Menu parse(std::string text)
		{
			text += '#';

			Menu menu{ {}, 0, false };
			size_t at = 0;

			while (at + 1 < text.size() && text[at] == '#')
			{
				size_t id_end = text.find('#', at + 1);
				size_t label_end = id_end == std::string::npos ? id_end : text.find('#', id_end + 1);

				if (label_end == std::string::npos)
					break;

				int32_t id = std::atoi(text.substr(at + 1, id_end - at - 1).c_str());
				menu.entries.push_back({ id, text.substr(id_end + 1, label_end - id_end - 1) });
				at = label_end;
			}

			// An id of -1 means nowhere to go. With other entries, before or after sorting, it leaves only an
			// unlabelled -1; alone, it keeps its label.
			auto nowhere = [&menu]()
			{
				return menu.entries.size() > 1 && menu.entries.front().id == -1;
			};

			if (!nowhere())
			{
				std::stable_sort(menu.entries.begin(), menu.entries.end(), [](const Entry &a, const Entry &b)
				{
					return a.id < b.id;
				});
			}

			if (nowhere())
				menu.entries = { { -1, "" } };

			menu.start = menu.entries.size() > 1 ? 1 : 0;
			menu.empty = menu.entries.empty() || menu.entries.front().id == -1;

			return menu;
		}
	}
}
