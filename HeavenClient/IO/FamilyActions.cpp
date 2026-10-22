#include "FamilyActions.h"

#include "UI.h"

#include "UITypes/UIChatBar.h"
#include "UITypes/UIFamily.h"
#include "UITypes/UIFamilyTree.h"
#include "UITypes/UINotice.h"

#include "../Gameplay/Stage.h"
#include "../Net/Packets/FamilyPackets.h"

namespace ms
{
	namespace FamilyActions
	{
		namespace
		{
			// StringPool 0x11e9 and 0x11f5.
			constexpr const char* ADD_PROMPT = "Enter the name of the character you wish to add as Junior. You must be in the same channel as well as map to add.";
			constexpr const char* CANNOT_ADD = "You cannot add this character as a Junior.";
			// A character's name.
			constexpr size_t NAME_LIMIT = 12;
		}

		void toggle_window()
		{
			UIFamily::toggle();
		}

		void open_tree(const std::string& name)
		{
			auto tree = UI::get().get_element<UIFamilyTree>();

			if (!tree || !tree->is_active())
				UI::get().emplace<UIFamilyTree>();

			OpenFamilyPedigreePacket(name).dispatch();
		}

		void prompt_junior()
		{
			UI::get().emplace<UIEnterText>(ADD_PROMPT, [](const std::string& name)
			{
				add_junior(name);
			}, NAME_LIMIT);
		}

		void add_junior(const std::string& name)
		{
			Player& player = Stage::get().get_player();

			if (!player.get_family().loaded())
				return;

			if (name == player.get_stats().get_name())
			{
				UI::get().emplace<UIOk>(CANNOT_ADD, [](bool) {});

				return;
			}

			AddFamilyPacket(name).dispatch();
		}

		bool can_add_junior()
		{
			const Family& family = Stage::get().get_player().get_family();
			const FamilyInfo& info = family.get_info();

			return family.loaded() && info.juniors < info.max_juniors;
		}

		void chat_line(const std::string& text)
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(text, UIChatBar::LineType::SKILL);
		}
	}
}
