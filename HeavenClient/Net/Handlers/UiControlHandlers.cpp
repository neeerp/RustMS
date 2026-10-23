#include "UiControlHandlers.h"

#include "../Packets/PlayerInteractionPackets.h"

#include "../../Gameplay/Stage.h"
#include "../../IO/FamilyActions.h"
#include "../../IO/UI.h"
#include "../../IO/UiControl.h"
#include "../../IO/UITypes/UICharInfo.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UIEnergyBar.h"
#include "../../IO/UITypes/UIEquipInventory.h"
#include "../../IO/UITypes/UIFamily.h"
#include "../../IO/UITypes/UIFamilyTree.h"
#include "../../IO/UITypes/UIGuildBBS.h"
#include "../../IO/UITypes/UIImageSlides.h"
#include "../../IO/UITypes/UIItemInventory.h"
#include "../../IO/UITypes/UIKeyConfig.h"
#include "../../IO/UITypes/UIMaker.h"
#include "../../IO/UITypes/UIMonsterBook.h"
#include "../../IO/UITypes/UINotice.h"
#include "../../IO/UITypes/UIPartySearch.h"
#include "../../IO/UITypes/UIQuestLog.h"
#include "../../IO/UITypes/UISkillBook.h"
#include "../../IO/UITypes/UIStatsInfo.h"

namespace ms
{
	namespace
	{
		// StringPool 0x1241.
		const char* const RANKING_NOTICE =
				"The Ranking Information can be referenced when your\\ncharacter changes occupation or reaches above Lv. 10.";

		template <class T>
		bool is_open()
		{
			auto window = UI::get().get_element<T>();

			return window && window->is_active();
		}

		// v83 makes the window only when its slot is empty, so an open one
		// stays as it is.
		template <class T, class... Args>
		void open(Args&&... args)
		{
			if (!is_open<T>())
				UI::get().emplace<T>(std::forward<Args>(args)...);
		}
	}

	void OpenUiHandler::handle(InPacket& recv) const
	{
		int8_t id = recv.read_byte();

		Stage& stage = Stage::get();
		Player& player = stage.get_player();
		const CharStats& stats = player.get_stats();

		ui_control::Window window = ui_control::open_window(id, stats.get_stat(MapleStat::Id::JOB),
				stats.get_stat(MapleStat::Id::LEVEL), stage.get_mapinfo().get_fieldtype());

		switch (window)
		{
			case ui_control::Window::ITEMS:
				open<UIItemInventory>(player.get_inventory());
				break;
			case ui_control::Window::EQUIPS:
				open<UIEquipInventory>(player.get_inventory());
				break;
			case ui_control::Window::STATS:
				open<UIStatsInfo>(stats);
				break;
			case ui_control::Window::SKILLS:
				open<UISkillBook>(stats, player.get_skills());
				break;
			case ui_control::Window::KEYCONFIG:
				open<UIKeyConfig>(player.get_inventory(), player.get_skills());
				break;
			case ui_control::Window::QUESTLOG:
				open<UIQuestLog>(player.get_quests());
				break;
			case ui_control::Window::MONSTERBOOK:
				open<UIMonsterBook>(player.get_monsterbook());
				break;
			case ui_control::Window::CHARINFO:
				// The player's own, asked for as a click on them is.
				if (!is_open<UICharInfo>() && player.request(CharInfoRequestPacket(player.get_oid()), ExclRequest::USE_WAIT))
					UI::get().emplace<UICharInfo>(player.get_oid());
				break;
			case ui_control::Window::GUILDBBS:
				UIGuildBBS::open();
				break;
			case ui_control::Window::ENERGYBAR:
				open<UIEnergyBar>(player.get_energy());
				break;
			case ui_control::Window::PARTYSEARCH:
				if (!is_open<UIPartySearch>())
					UIPartySearch::toggle();
				break;
			case ui_control::Window::MAKER:
				open<UIMaker>();
				break;
			case ui_control::Window::FAMILY:
				if (!is_open<UIFamily>())
					UIFamily::toggle();
				break;
			case ui_control::Window::FAMILYTREE:
				if (!is_open<UIFamilyTree>())
					FamilyActions::open_tree(stats.get_name());
				break;
			case ui_control::Window::RANKING_NOTICE:
				UI::get().emplace<UIOk>(RANKING_NOTICE, [](bool) {});
				break;
			case ui_control::Window::NONE:
				break;
		}
	}

	void LockUiHandler::handle(InPacket& recv) const
	{
		UI::get().lock(recv.read_byte() != 0);
	}

	void DisableUiHandler::handle(InPacket& recv) const
	{
		ui_control::state().disabled = recv.read_byte() != 0;
	}

	void MessageBoxHandler::handle(InPacket& recv) const
	{
		std::string text = recv.read_string();

		// v83 drops it while the HUD is gone, with the status bar that holds
		// the chat log (FUN_004906b5).
		if (ui_control::state().locked)
			return;

		if (auto chatbar = UI::get().get_element<UIChatBar>())
			chatbar->send_chatline(text, UIChatBar::LineType::SKILL);
	}

	void OpenUiStringListHandler::handle(InPacket& recv) const
	{
		uint8_t count = static_cast<uint8_t>(recv.read_byte());
		std::vector<std::string> paths;

		for (uint8_t i = 0; i < count; i++)
			paths.push_back(recv.read_string());

		// The modal dialog takes the keys from a chat line being typed (FUN_004edba1).
		UI::get().remove_textfield();
		UI::get().emplace<UIImageSlides>(paths);
	}
}
