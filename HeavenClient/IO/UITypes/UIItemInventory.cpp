//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "UIItemInventory.h"

#include "UIApReset.h"
#include "UISpReset.h"
#include "UIChatBar.h"
#include "UIItemMegaphone.h"
#include "UIMegaphone.h"
#include "UINotice.h"
#include "UIPersonalShop.h"
#include "UIPetChooser.h"
#include "UITeleportRock.h"
#include "UITradingRoom.h"

#include "../Messages.h"
#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/MapleButton.h"
#include "../UITypes/UIKeyConfig.h"

#include "../../Character/GachaExp.h"
#include "../../Character/Pets.h"
#include "../../Character/Inventory/ItemDrop.h"
#include "../../Character/Inventory/ScrollRules.h"
#include "../../Data/CurseData.h"
#include "../../Data/EquipData.h"
#include "../../Data/PetData.h"
#include "../../Gameplay/Stage.h"
#include "../../Gameplay/WeatherRules.h"
#include "../../Gameplay/MapleMap/FieldLimit.h"
#include "../../Util/Chalkboard.h"
#include "../../Util/Megaphones.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Handlers/MiniRoom.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Net/Packets/MiniRoomPackets.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The window, measured from period footage. The full window keeps the
		// small one's left side; its title buttons follow the right edge.
		constexpr int16_t CLOSE_RIGHT = 19;
		constexpr int16_t TITLE_BUTTON_PITCH = 15;
		constexpr int16_t TITLE_BUTTON_Y = 12;
		constexpr Point<int16_t> TAB_STRIP = Point<int16_t>(3, 23);
		constexpr int16_t TAB_FILL = 22;
		// Each tab's highlight for a new item; the selected tab's is a pixel taller.
		constexpr Point<int16_t> NEW_TAB = Point<int16_t>(0, 1);
		// The top-left of the first slot, and the distance between slots.
		constexpr Point<int16_t> SLOT = Point<int16_t>(8, 50);
		constexpr Point<int16_t> FULL_SLOT = Point<int16_t>(7, 50);
		constexpr Point<int16_t> SLOT_PITCH = Point<int16_t>(36, 34);
		constexpr int16_t PAGE_PITCH = 149;
		constexpr int16_t SLIDER_X = 152;
		constexpr Range<int16_t> SLIDER_RANGE = Range<int16_t>(51, 237);
		constexpr Point<int16_t> COIN = Point<int16_t>(8, 267);
		constexpr Point<int16_t> FULL_COIN = Point<int16_t>(7, 267);
		// The right end of the meso amount, which sits in a field ending at 137.
		constexpr Point<int16_t> MESO = Point<int16_t>(131, 260);
		constexpr Point<int16_t> FULL_MESO = Point<int16_t>(135, 260);

		// v83's notices 0x10a and 0x42 (FUN_004f5697).
		constexpr const char *CATCH_BREATH = "You can't do it right now.\\nCatch your breath and then try again.";
		constexpr const char *DEAD = "You can't do it once you're dead.";
		// StringPool 0x129.
		constexpr const char *REQUEST_BUSY = "Other requests are in process.\\nPlease try again in a little bit.";
		// StringPool 0x180 and 0x181.
		constexpr const char *TOO_MANY_PETS = "You have exceeded the maximum number\\n of pets that can accompany you.\\nPlease try again after \\nyou change the living pet back into a doll.";
		// StringPool 0xed8.
		constexpr const char *CASH_FOOD_NO_PET = "This item can only be used with a pet.";
		// StringPool 0x319 to 0x31c, 0x110 and 0x335.
		constexpr const char *PET_HAS_NAME = "The pet already has a name.\\nWill you change it?";
		constexpr const char *NAME_PET = "Please name your pet.";
		constexpr const char *NAME_TAKEN = "That name is not available.";
		constexpr const char *NO_CURSING = "Cursing is not allowed.";
		// StringPool 0x112 and 0x318.
		constexpr const char *WEATHER_BUSY = "The effect is taking place right now.\\nPlease try it later.";
		constexpr const char *WEATHER_PROMPT = "Please enter the attached message";
		// StringPool 0x10f, 0xea1 and 0x111.
		constexpr const char *MEGAPHONE_PROMPT = "Enter your comments through the megaphone.";
		constexpr const char *MEGAPHONE_LEVEL = "This megaphone is only available for \\ncharacters that are over Level 10.";
		constexpr const char *MEGAPHONE_LONG = "Use the megaphone to text up to 60 characters.";
		constexpr const char *SUMMON_SACK = "Too much interest can hinder your process of leveling up.\\n\\nwill you really summon the monster?";
		constexpr const char *BOSS_PET = "Would you like to change your Boss Pet?\\n (The \"Item Ignore\" skill, as well as \\nthe stats for speed and jump will \\nfollow that of the Lead Pet)";

		// v83 FUN_00a240a2: double-clicking a pet asks the server to bring it
		// out, or back if it is out.
		void summon_pet(int16_t slot, int64_t serial)
		{
			Player &player = Stage::get().get_player();

			if (!player.can_request(ExclRequest::USE_WAIT))
				return;

			if (player.get_look().get_alerted())
			{
				UI::get().emplace<UIOk>(CATCH_BREATH, [](bool)
				{});
				return;
			}

			int32_t lead = pets::follow_the_lead(player.get_stats().get_job().get_id());

			switch (pets::summon(player.get_skilllevel(lead) > 0, player.get_pet_serials(), serial))
			{
				case pets::Summon::SEND:
					player.request(SpawnPetPacket(slot, false), ExclRequest::USE_WAIT);
					break;
				case pets::Summon::FULL:
					UI::get().emplace<UIOk>(TOO_MANY_PETS, [](bool)
					{});
					break;
				case pets::Summon::ASK_BOSS:
					// Any answer but Yes brings it out behind the others.
					UI::get().emplace<UIYesNo>(BOSS_PET, [slot](bool yes)
					{
						Stage::get().get_player().request(SpawnPetPacket(slot, yes), ExclRequest::USE_WAIT);
					});
					break;
			}
		}

		// v83 FUN_00a0a63f, type 0x1e: cash pet food needs a pet out, and the
		// server picks the pet it suits. Using it plays the shop sound; the
		// pet's eat sound waits for CASH_PET_FOOD_RESULT.
		void feed_cash_food(int16_t slot, int32_t item_id)
		{
			Player &player = Stage::get().get_player();

			if (!player.can_request(ExclRequest::WAIT))
				return;

			if (!player.get_pet_serials()[0])
			{
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(CASH_FOOD_NO_PET, UIChatBar::LineType::SKILL);

				return;
			}

			if (player.request(UseCashItemPacket(slot, item_id), ExclRequest::WAIT))
				Sound(nl::nx::sound["Game.img"]["UseShopItem"]).play();
		}

		// v83 FUN_00a0a63f, type 0x11, after asking whether to rename a pet
		// that has a name: the name, then whether to use it. A name too
		// short asks again; anything else refused ends it.
		void ask_pet_name(int16_t slot, int32_t item_id, const std::string &species, const std::string &current)
		{
			UI::get().emplace<UIEnterText>(NAME_PET, [=](const std::string &typed)
			{
				std::string name = typed;

				switch (pets::check_name(name, species, current, CurseData::filter))
				{
					case pets::Naming::OK:
						UI::get().emplace<UIYesNo>("Do you want to name your pet\\n'" + name + "'?", [=](bool yes)
						{
							if (yes && Stage::get().get_player().request(UseCashItemPacket(slot, item_id, name), ExclRequest::WAIT))
								Sound(nl::nx::sound["Game.img"]["UseShopItem"]).play();
						});
						break;
					case pets::Naming::SHORT:
						UI::get().emplace<UIOk>("Please enter at least " + std::to_string(pets::MIN_NAME) + " characters.", [=](bool)
						{
							ask_pet_name(slot, item_id, species, current);
						});
						break;
					case pets::Naming::CURSING:
						UI::get().emplace<UIOk>(NO_CURSING, [](bool)
						{});
						break;
					case pets::Naming::TAKEN:
						UI::get().emplace<UIOk>(NAME_TAKEN, [](bool)
						{});
						break;
				}
			}, pets::MAX_NAME);
		}

		// v83 FUN_00a0a63f, type 0x11: the Pet Name Tag names the first pet
		// out. Without one it does nothing. A pet already renamed asks first.
		void name_pet(int16_t slot, int32_t item_id)
		{
			Player &player = Stage::get().get_player();

			if (!player.can_request(ExclRequest::WAIT))
				return;

			Optional<const Pet> pet = player.get_inventory().get_pet(player.get_pet_serials()[0]);

			if (!pet)
				return;

			std::string species = nl::nx::string["Pet.img"][std::to_string(pet->get_item_id())]["name"];
			std::string current = pet->get_name();

			if (pets::same_name(species, current))
				ask_pet_name(slot, item_id, species, current);
			else
				UI::get().emplace<UIYesNo>(PET_HAS_NAME, [=](bool yes)
				{
					if (yes)
						ask_pet_name(slot, item_id, species, current);
				});
		}

		// v83 FUN_00a0a63f waits for the last request before a cash item
		// opens its window, and again before it is used, saying so.
		bool may_use_cash_item()
		{
			if (Stage::get().get_player().can_request(ExclRequest::WAIT))
				return true;

			UI::get().emplace<UIOk>(REQUEST_BUSY, [](bool)
			{});

			return false;
		}

		// Some cash items say so in the chat log while a shop or trade is open.
		bool in_mini_room()
		{
			auto shop = UI::get().get_element<UIPersonalShop>();
			auto trade = UI::get().get_element<UITradingRoom>();

			if ((!shop || !shop->is_active()) && (!trade || !trade->is_active()))
				return false;

			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline("Not available", UIChatBar::LineType::SKILL);

			return true;
		}

		// v83 FUN_00a0a63f, type 0x20: a chalkboard asks for a note, which
		// goes up once the server sends it back. A blank note isn't sent.
		void use_chalkboard(int16_t slot, int32_t item_id)
		{
			if (!may_use_cash_item() || in_mini_room())
				return;

			if (Stage::get().get_mapinfo().allows_personal_shop())
			{
				UI::get().emplace<UIOk>("You can't use it here in this map.", [](bool)
				{});
				return;
			}

			UI::get().emplace<UIEnterText>("Enter a note.", [=](const std::string &text)
			{
				if (!Chalkboard::trim(text).empty() && may_use_cash_item())
					Stage::get().get_player().request(UseCashItemPacket(slot, item_id, text), ExclRequest::WAIT);
			}, Chalkboard::MAX_TEXT);
		}

		// v83 FUN_00a0a63f, type 0x10: a weather item asks for a message of
		// up to 35 characters, unless the field's weather isn't over or the
		// map forbids them. A blank message isn't sent.
		void use_weather(int16_t slot, int32_t item_id)
		{
			if (!may_use_cash_item())
				return;

			const MapInfo &mapinfo = Stage::get().get_mapinfo();

			if (Stage::get().weather_busy()
				|| !field_limit::may_blow_weather(mapinfo.get_fieldlimit(), mapinfo.get_fieldtype()))
			{
				UI::get().emplace<UIOk>(WEATHER_BUSY, [](bool)
				{});
				return;
			}

			UI::get().emplace<UIEnterText>(WEATHER_PROMPT, [=](const std::string &text)
			{
				if (text.empty())
					return;

				if (CurseData::filter(text) != text)
				{
					UI::get().emplace<UIOk>(NO_CURSING, [](bool)
					{});
					return;
				}

				if (may_use_cash_item())
					Stage::get().get_player().request(UseCashItemPacket(slot, item_id, text), ExclRequest::WAIT);
			}, weather::MAX_MESSAGE);
		}

		// v83 FUN_00a0a63f, type 0x17: AP Reset's dialogs pick where an AP
		// leaves and where it goes. A GM can't use it, and says nothing.
		void use_ap_reset(int16_t slot, int32_t item_id)
		{
			if (!may_use_cash_item() || in_mini_room())
				return;

			if (!ap_reset::may_use(Stage::get().get_player().get_stats().get_stat(MapleStat::Id::JOB)))
				return;

			UI::get().emplace<UIApReset>([=](int32_t to, int32_t from)
			{
				if (may_use_cash_item())
					Stage::get().get_player().request(UseResetPacket(slot, item_id, to, from), ExclRequest::WAIT);
			});
		}

		// Type 0x18: SP Reset's dialogs pick the skill a level leaves and the
		// one of the item's advancement it goes to. Beginners, GMs and those
		// not advanced as far as the item can't use it, and are told nothing.
		void use_sp_reset(int16_t slot, int32_t item_id)
		{
			if (!may_use_cash_item() || in_mini_room())
				return;

			int32_t job = Stage::get().get_player().get_stats().get_stat(MapleStat::Id::JOB);

			if (!sp_reset::may_use(job, sp_reset::item_tab(item_id)))
				return;

			UI::get().emplace<UISpReset>(item_id, [=](int32_t to, int32_t from)
			{
				if (may_use_cash_item())
					Stage::get().get_player().request(UseResetPacket(slot, item_id, to, from), ExclRequest::WAIT);
			});
		}

		// Whether a megaphone's line is refused, saying why if v83 does.
		bool megaphone_refused(Megaphones::Refusal refusal)
		{
			const char *reason = nullptr;

			switch (refusal)
			{
				case Megaphones::Refusal::NONE:
					return false;
				case Megaphones::Refusal::CURSING:
					reason = NO_CURSING;
					break;
				case Megaphones::Refusal::LEVEL:
					reason = MEGAPHONE_LEVEL;
					break;
				case Megaphones::Refusal::LONG:
					reason = MEGAPHONE_LONG;
					break;
				case Megaphones::Refusal::QUIET:
					break;
			}

			if (reason)
				UI::get().emplace<UIOk>(reason, [](bool)
				{});

			return true;
		}

		// Checks a plain or super megaphone's line with the level I have now.
		bool line_refused(const std::string &text)
		{
			uint16_t level = Stage::get().get_player().get_stats().get_stat(MapleStat::Id::LEVEL);

			return megaphone_refused(Megaphones::check_line(text, level, CurseData::filter));
		}

		void send_megaphone(OutPacket &&packet)
		{
			if (may_use_cash_item() && Stage::get().get_player().request(std::move(packet), ExclRequest::WAIT))
				Sound(nl::nx::sound["Game.img"]["UseShopItem"]).play();
		}

		// v83 FUN_00a0a63f, types 0xc to 0xf and 0x3c: a megaphone asks for
		// its line, and checks it once the prompt has closed. The item
		// megaphone's window stays open beside the game and sends by itself.
		void use_megaphone(int16_t slot, int32_t item_id)
		{
			if (!may_use_cash_item())
				return;

			switch (Megaphones::kind(item_id))
			{
				case Megaphones::Kind::PLAIN:
					UI::get().emplace<UIEnterText>(MEGAPHONE_PROMPT, [=](const std::string &text)
					{
						if (!line_refused(text))
							send_megaphone(UseCashItemPacket(slot, item_id, text));
					}, Megaphones::MAX_LINE);
					break;
				case Megaphones::Kind::SUPER:
					UI::get().emplace<UIMegaphone>(item_id, [=](const std::vector<std::string> &lines, bool ear)
					{
						if (!line_refused(lines[0]))
							send_megaphone(UseSuperMegaphonePacket(slot, item_id, lines[0], ear));
					});
					break;
				case Megaphones::Kind::TRIPLE:
					UI::get().emplace<UIMegaphone>(item_id, [=](const std::vector<std::string> &lines, bool ear)
					{
						if (!megaphone_refused(Megaphones::check_lines(lines)))
							send_megaphone(UseTripleMegaphonePacket(slot, item_id, Megaphones::lines_sent(lines), ear));
					});
					break;
				case Megaphones::Kind::ITEM:
					if (!in_mini_room())
						UI::get().emplace<UIItemMegaphone>(slot, item_id);

					break;
				case Megaphones::Kind::NONE:
					break;
			}
		}

		// v83 FUN_00a0a4aa: a teleport rock goes to the map or the
		// character chosen in its list. The dead, and maps that don't allow
		// teleports, are refused before the list opens. The USE-tab rock
		// (232xxxx, FUN_00a0a3bb) waits as an item use does, and says nothing
		// while another request waits.
		bool may_use_rock(int32_t item_id)
		{
			return item_id / 10000 == 504
				? may_use_cash_item()
				: Stage::get().get_player().can_request(ExclRequest::USE_WAIT);
		}

		void use_teleport_rock(int16_t slot, int32_t item_id)
		{
			if (!may_use_rock(item_id))
				return;

			if (Stage::get().get_player().is_dead())
			{
				UI::get().emplace<UIOk>("You can't do it while you're dead.", [](bool)
				{});
				return;
			}

			if (!field_limit::may_teleport_rock(Stage::get().get_mapinfo().get_fieldlimit(), Stage::get().get_mapid()))
			{
				UI::get().emplace<UIOk>("You cannot teleport to this map.", [](bool)
				{});
				return;
			}

			UI::get().emplace<UITeleportRock>(TeleportRock::is_vip(item_id), [slot, item_id](const std::string &name, int32_t map_id)
			{
				Player &player = Stage::get().get_player();
				player.get_teleport_rock().set_target(name);

				if (name.empty() && (map_id == TeleportRock::EMPTY || map_id == 0))
					return;

				if (!may_use_rock(item_id))
					return;

				int32_t wait = item_id / 10000 == 504 ? ExclRequest::WAIT : ExclRequest::USE_WAIT;
				bool sent = name.empty()
						? player.request(UseTeleportRockPacket(slot, item_id, map_id), wait)
						: player.request(UseTeleportRockPacket(slot, item_id, name), wait);

				if (sent)
					Sound(nl::nx::sound["Game.img"]["UseShopItem"]).play();
			});
		}

		// v83 FUN_00a1e1ce: a return scroll (203xxxx) goes to its spec/moveTo,
		// or to the map's return map, once the map and the area allow it.
		void use_return_scroll(int16_t slot, int32_t item_id)
		{
			static const std::map<int32_t, int32_t> area_codes = []()
			{
				std::map<int32_t, int32_t> codes;

				for (nl::node code : nl::nx::map["Map"]["AreaCode.img"])
					codes[std::stoi(code.name())] = static_cast<int32_t>(code.get_integer());

				return codes;
			}();

			Player &player = Stage::get().get_player();

			if (!player.can_request(ExclRequest::WAIT))
				return;

			nl::node spec = nl::nx::item["Consume"]["0203.img"]["0" + std::to_string(item_id)]["spec"];
			int32_t move_to = spec["moveTo"].get_integer(field_limit::NEAREST_TOWN);
			bool ignore_continent = spec["ignoreContinent"].get_integer() != 0;

			switch (field_limit::return_scroll_refusal(Stage::get().get_mapinfo().get_fieldlimit(), Stage::get().get_mapid(),
					move_to, ignore_continent, area_codes))
			{
				case field_limit::ScrollRefusal::HERE:
					UI::get().emplace<UIOk>("You can't use it here in this map.", [](bool)
					{});
					break;
				case field_limit::ScrollRefusal::THERE:
					UI::get().emplace<UIOk>("You cannot go to that place.", [](bool)
					{});
					break;
				case field_limit::ScrollRefusal::NONE:
					if (player.request(ReturnScrollPacket(slot, item_id), ExclRequest::WAIT))
						Sound(item_id).play();

					break;
			}
		}

		// v83 FUN_00a0977b: a summoning sack (210xxxx) stays shut while
		// riding or where the map forbids it, and asks first unless it is an
		// event's.
		void use_summon_sack(int16_t slot, int32_t item_id)
		{
			if (Stage::get().get_player().ride_refuses())
				return;

			if (!Stage::get().get_mapinfo().can_summon(item_id))
			{
				UI::get().emplace<UIOk>("You can't use it here in this map.", [](bool)
				{});
				return;
			}

			auto open = [slot, item_id]()
			{
				Stage::get().get_player().request(SummonSackPacket(slot, item_id), ExclRequest::USE_WAIT);
			};

			if (!field_limit::sack_asks(item_id))
				open();
			else
				UI::get().emplace<UIYesNo>(SUMMON_SACK, [open](bool yes)
				{
					if (yes)
						open();
				});
		}

		// v83 FUN_00a12685: a Writ of Solomon (237xxxx) becomes gacha EXP, up
		// to the writ's info/maxLevel and only once the last is taken, and
		// not on a tamed monster.
		void use_solomon_writ(int16_t slot, int32_t item_id)
		{
			Player &player = Stage::get().get_player();

			if (player.ride_refuses(true))
				return;

			const CharStats &stats = player.get_stats();
			nl::node info = nl::nx::item["Consume"]["0237.img"]["0" + std::to_string(item_id)]["info"];
			int32_t max_level = info["maxLevel"].get_integer();

			switch (gacha_exp::writ_refusal(stats.get_stat(MapleStat::Id::LEVEL), max_level, stats.get_gachaexp()))
			{
				case gacha_exp::WritRefusal::LEVEL:
					InChatMessage(Messages::ITEM_LEVEL_HIGH).drop();
					break;
				case gacha_exp::WritRefusal::EXP_LEFT:
					InChatMessage(Messages::SOLOMON_EXP_LEFT).drop();
					break;
				case gacha_exp::WritRefusal::NONE:
					if (player.request(SolomonItemPacket(slot, item_id), ExclRequest::WAIT))
						Sound(item_id).play();

					break;
			}
		}

		// v83 FUN_00a0900a: a map may keep items off the ground, which it
		// says only once the player has answered every question.
		void send_drop(InventoryType::Id tab, int16_t slot, int16_t qty)
		{
			if (Stage::get().get_mapinfo().can_drop())
				Stage::get().get_player().request(MoveItemPacket(tab, slot, 0, qty), ExclRequest::WAIT);
			else
				InChatMessage(Messages::DROP_FIELD).drop();
		}

		// Put up `steps` from `next` on, then do `then` if every question had Yes for an answer.
		void confirm(std::vector<item_drop::Step> steps, size_t next, std::function<void()> then)
		{
			if (next == steps.size())
			{
				then();

				return;
			}

			item_drop::Step step = steps[next];

			if (step.question)
			{
				auto onanswer = [steps, next, then](bool yes)
				{
					if (yes)
						confirm(steps, next + 1, then);
				};

				UI::get().emplace<UIYesNo>(step.text, onanswer);
			} else
			{
				UI::get().emplace<UIOk>(step.text, [](bool) {});
			}
		}

		// Ask to wear the equip in Equip tab `slot` at `worn`, after asking first where wearing it makes it
		// untradeable (v83 FUN_004f1c2d, last of its checks).
		void put_on(int16_t slot, int16_t worn)
		{
			auto send = [slot, worn]()
			{
				Stage::get().get_player().request(EquipItemPacket(slot, worn), ExclRequest::WAIT);
			};

			const Inventory &inventory = Stage::get().get_player().get_inventory();
			Optional<const Equip> equip = inventory.get_equip(InventoryType::Id::EQUIP, slot);

			if (Stage::get().get_player().ride_keeps(inventory.get_item_id(InventoryType::Id::EQUIP, slot)))
				return;

			if (equip && ItemData::get(equip->get_item_id()).get_properties().equip_trade_block
				&& !(equip->get_flags() & item_properties::EQUIP_UNTRADEABLE))
			{
				auto onanswer = [send](bool yes)
				{
					if (yes)
						send();
				};

				UI::get().emplace<UIYesNo>("You cannot trade this item after equipping.\\n Do you still wish to equip?",
					onanswer);
			} else
			{
				send();
			}
		}
	}

	UIItemInventory::UIItemInventory(const Inventory &invent)
			: UIDragElement<PosINV>(), inventory(invent), tab(InventoryType::Id::EQUIP), ignore_tooltip(false),
			  sort_enabled(false), full_enabled(false)
	{
		nl::node Item = NxFiles::ui_classic["UIWindow.img"]["Item"];

		backgrnd = Item["backgrnd"];
		full_backgrnd = Item["FullBackgrnd"];

		tabs = ClassicTabs(ClassicTabs::TAB5, Item["Tab"]["enabled"], Item["Tab"]["disabled"], TAB_FILL);

		nl::node New = Item["New"];
		newitemslot = New["inventory"];
		newitemtab[false] = New["Tab0"];
		newitemtab[true] = New["Tab1"];

		projectile = Item["activeIcon"];
		disabled = Item["disabled"];

		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"]);

		for (uint16_t i = 0; i < TABS; i++)
		{
			Point<int16_t> tab_pos = TAB_STRIP + Point<int16_t>(tabs.pitch() * i, 0);

			buttons[Buttons::BT_TAB0 + i] = std::make_unique<AreaButton>(tab_pos,
																		 Point<int16_t>(tabs.pitch(), tabs.height()));
			buttons[Buttons::BT_TAB0 + i]->set_tab();
		}

		buttons[Buttons::BT_COIN] = std::make_unique<MapleButton>(Item["BtCoin"]);
		buttons[Buttons::BT_GATHER] = std::make_unique<MapleButton>(Item["BtGather"]);
		buttons[Buttons::BT_SORT] = std::make_unique<MapleButton>(Item["BtSort"]);
		buttons[Buttons::BT_FULL] = std::make_unique<MapleButton>(Item["BtFull"]);
		buttons[Buttons::BT_SMALL] = std::make_unique<MapleButton>(Item["BtSmall"]);

		mesolabel = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);

		for (uint16_t i = 0; i < TABS; i++)
			rows[static_cast<InventoryType::Id>(InventoryType::Id::EQUIP + i)] = 0;

		slider = Slider(
				Slider::Type::CLASSIC, SLIDER_RANGE, SLIDER_X, ROWS, 1,
				[&](bool upwards)
				{
					int16_t row = rows[tab] + (upwards ? -1 : 1);
					int16_t last = (inventory.get_slotmax(tab) + COLUMNS - 1) / COLUMNS - ROWS;

					if (row >= 0 && row <= last)
						rows[tab] = row;
				}
		);

		set_full(false);
		update_slider();
		clear_new();
	}

	void UIItemInventory::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		if (full_enabled)
			full_backgrnd.draw(position);
		else
			backgrnd.draw(position);

		uint16_t selected = tab - InventoryType::Id::EQUIP;
		tabs.draw(position + TAB_STRIP, TABS, selected);

		if (newtab != InventoryType::Id::NONE)
		{
			uint16_t index = newtab - InventoryType::Id::EQUIP;
			bool is_selected = index == selected;
			Point<int16_t> tab_pos = TAB_STRIP + Point<int16_t>(tabs.pitch() * index, 0) + NEW_TAB;

			if (is_selected)
				tab_pos.shift_y(-1);

			newitemtab[is_selected].draw(position + tab_pos, alpha);
			tabs.draw_label(position + TAB_STRIP, index, is_selected);
		}

		if (full_enabled)
		{
			mesolabel.draw(position + FULL_MESO);
		} else
		{
			slider.draw(position);
			mesolabel.draw(position + MESO);
		}

		int16_t numslots = inventory.get_slotmax(tab);

		for (int16_t slot = first_slot(); slot <= last_slot(); slot++)
		{
			Point<int16_t> slotpos = position + get_slotpos(slot);

			if (slot > numslots)
			{
				disabled.draw(slotpos);
				continue;
			}

			auto iter = icons.find(slot);

			if (iter != icons.end() && iter->second)
				iter->second->draw(slotpos);
		}

		int16_t bulletslot = inventory.get_bulletslot();

		if (tab == InventoryType::Id::USE && is_visible(bulletslot))
			projectile.draw(position + get_slotpos(bulletslot));

		if (tab == newtab && is_visible(newslot))
			newitemslot.draw(position + get_slotpos(newslot), alpha);

		UIElement::draw_buttons(alpha);
	}

	void UIItemInventory::update()
	{
		UIElement::update();

		newitemtab[false].update(6);
		newitemtab[true].update(6);
		newitemslot.update(6);

		std::string meso_str = std::to_string(inventory.get_meso());
		string_format::split_number(meso_str);

		mesolabel.change_text(meso_str);
	}

	void UIItemInventory::update_slot(int16_t slot)
	{
		if (int32_t item_id = inventory.get_item_id(tab, slot))
		{
			int16_t count;

			if (tab == InventoryType::Id::EQUIP)
				count = -1;
			else
				count = inventory.get_item_count(tab, slot);

			const ItemData &idata = ItemData::get(item_id);
			const bool cashitem = idata.is_cashitem() || inventory.get_cash_id(tab, slot) != 0;
			int16_t flags = 0;
			int64_t expiration = item_expiry::BEFORE;

			if (Optional<const Equip> equip = inventory.get_equip(tab, slot))
			{
				flags = equip->get_flags();
				expiration = equip->get_expiration();
			} else if (Optional<const Item> item = inventory.get_item(tab, slot))
			{
				flags = item->get_flags();
				expiration = item->get_expiration();
			}

			item_drop::Traits traits = item_drop::traits(idata.get_properties(), tab == InventoryType::Id::EQUIP,
				cashitem, flags, expiration);
			const Texture &texture = idata.get_icon(false);
			EquipSlot::Id eqslot = inventory.find_equipslot(item_id);

			icons[slot] = std::make_unique<Icon>(
					std::make_unique<ItemIcon>(*this, tab, eqslot, slot, item_id, count, traits),
					texture, count
			);
		} else if (icons.count(slot))
		{
			icons.erase(slot);
		}
	}

	void UIItemInventory::load_icons()
	{
		icons.clear();

		int16_t numslots = inventory.get_slotmax(tab);

		for (int16_t slot = 1; slot <= numslots; slot++)
			update_slot(slot);
	}

	void UIItemInventory::refresh_slots()
	{
		load_icons();
		update_slider();
	}

	void UIItemInventory::update_slider()
	{
		int16_t total = (inventory.get_slotmax(tab) + COLUMNS - 1) / COLUMNS;
		slider.setrows(rows[tab], ROWS, std::max<int16_t>(total, ROWS));
	}

	Button::State UIItemInventory::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::BT_CLOSE:
				toggle_active();
				break;
			case Buttons::BT_TAB0:
			case Buttons::BT_TAB1:
			case Buttons::BT_TAB2:
			case Buttons::BT_TAB3:
			case Buttons::BT_TAB4:
			{
				auto type = static_cast<InventoryType::Id>(InventoryType::Id::EQUIP + buttonid - Buttons::BT_TAB0);

				if (type != tab)
				{
					clear_tooltip();

					tab = type;

					update_slider();
					load_icons();
					set_sort(false);
				}

				// The strip is drawn from tab, not the button states. A tab
				// left PRESSED would ignore every later click (#188).
				break;
			}
			case Buttons::BT_GATHER:
				Stage::get().get_player().request(GatherItemsPacket(tab), ExclRequest::WAIT);
				break;
			case Buttons::BT_SORT:
				Stage::get().get_player().request(SortItemsPacket(tab), ExclRequest::WAIT);
				break;
			case Buttons::BT_FULL:
				set_full(true);
				break;
			case Buttons::BT_SMALL:
				set_full(false);
				break;
			case Buttons::BT_COIN:
			{
				// One drop holds 10 to 50,000 mesos, and the box starts at 10.
				constexpr int32_t MESO_DROP_MIN = 10;
				constexpr int32_t MESO_DROP_MAX = 50000;

				auto most = static_cast<int32_t>(std::min<int64_t>(inventory.get_meso(), MESO_DROP_MAX));

				if (most >= MESO_DROP_MIN)
				{
					auto onenter = [](int32_t amount)
					{
						if (Stage::get().get_mapinfo().can_drop())
							Stage::get().get_player().request(MesoDropPacket(amount), ExclRequest::USE_WAIT);
						else
							InChatMessage(Messages::DROP_FIELD).drop();
					};

					UI::get().emplace<UIEnterNumber>(
							"How many will you drop? (A portion of the amount may be lost if picked up by another user.)",
							onenter, most, MESO_DROP_MIN, MESO_DROP_MIN);
				}

				break;
			}
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UIItemInventory::doubleclick(Point<int16_t> cursorpos)
	{
		int16_t slot = slot_by_position(cursorpos - position);

		if (icons.count(slot) && is_visible(slot))
		{
			if (int32_t item_id = inventory.get_item_id(tab, slot))
			{
				// v83 FUN_004efd25 refuses everything once the player is dead.
				if (Stage::get().get_player().is_dead())
				{
					UI::get().emplace<UIOk>(DEAD, [](bool)
					{});
					return;
				}

				switch (tab)
				{
					case InventoryType::Id::EQUIP:
						if (pets::equip_parts(item_id))
						{
							wear_pet_equip(slot, item_id);
						} else if (can_wear_equip(slot))
						{
							put_on(slot, inventory.equip_position(slot, inventory.find_equipslot(item_id)));
							Sound(Sound::Name::DRAGEND).play();
						}

						break;
					case InventoryType::Id::USE:
						if (item_id / 10000 == 203)
							use_return_scroll(slot, item_id);
						else if (item_id / 10000 == 210)
							use_summon_sack(slot, item_id);
						else if (item_id / 10000 == 232)
							use_teleport_rock(slot, item_id);
						else if (item_id / 10000 == 237)
							use_solomon_writ(slot, item_id);
						else if (item_id / 10000 == 226)
							Stage::get().get_player().feed_mount(slot, item_id);
						else if (!Stage::get().get_mapinfo().can_use_item(item_id))
							InChatMessage(Messages::ITEM_FIELD).drop();
						else if (field_limit::is_use_item(item_id) && Stage::get().get_player().ride_refuses(true))
							break;
						else if (!disease::can_use_item(Stage::get().get_player().get_diseases()))
							break;
						else if (pets::is_food(item_id))
							Stage::get().get_player().feed_pets(slot, item_id);
						else if (item_id / 10000 == 228 || item_id / 10000 == 229)
							Stage::get().get_player().request(SkillBookPacket(slot, item_id), ExclRequest::USE_WAIT);
						else if (field_limit::is_use_item(item_id))
							Stage::get().get_player().request(UseItemPacket(slot, item_id), ExclRequest::USE_WAIT);

						break;
					case InventoryType::Id::SETUP:
						Stage::get().get_player().use_item(item_id);
						break;
					case InventoryType::Id::CASH:
						// As in v83, a face item (516xxxx) makes its emote
						// (id % 100 + 8) as a face key would, without being
						// used up.
						if (item_id / 10000 == 516)
							Stage::get().send_key(KeyType::Id::FACE, 99 + item_id % 100 + 8, true);
						else if (personal_shop::is_permit(item_id))
							UIPersonalShop::create(slot, item_id);
						else if (item_id / 10000 == 500)
							summon_pet(slot, inventory.get_cash_id(tab, slot));
						else if (pets::is_cash_food(item_id))
							feed_cash_food(slot, item_id);
						else if (pets::is_name_tag(item_id))
							name_pet(slot, item_id);
						else if (item_id / 10000 == 537)
							use_chalkboard(slot, item_id);
						else if (item_id / 10000 == 504)
							use_teleport_rock(slot, item_id);
						else if (item_id / 10000 == 507)
							use_megaphone(slot, item_id);
						else if (weather::is_weather(item_id))
							use_weather(slot, item_id);
						else if (item_id == ap_reset::ITEM)
							use_ap_reset(slot, item_id);
						else if (sp_reset::is_item(item_id))
							use_sp_reset(slot, item_id);
						else if (item_id / 10000 == 501)
							Stage::get().use_item_effect(item_id);

						break;
					case InventoryType::Id::ETC:
						if (item_id / 10000 == 429)
							Stage::get().use_item_effect(item_id);

						break;
					default:
						break;
				}
			}
		}
	}

	bool UIItemInventory::send_icon(const Icon &icon, Point<int16_t> cursorpos)
	{
		// A row of my shop goes back anywhere on the window, once it shows
		// the row's tab (FUN_004f4b49).
		if (InventoryType::Id back = icon.get_return_tab(); back != InventoryType::Id::NONE)
		{
			if (back != tab)
			{
				change_tab(back);

				return false;
			}

			icon.drop_on_inventory();

			return true;
		}

		int16_t slot = slot_by_position(cursorpos - position);

		if (slot > 0)
		{
			int32_t item_id = inventory.get_item_id(tab, slot);
			EquipSlot::Id eqslot;
			bool equip;

			if (item_id && tab == InventoryType::Id::EQUIP)
			{
				eqslot = inventory.find_equipslot(item_id);
				equip = true;
			} else
			{
				eqslot = EquipSlot::Id::NONE;
				equip = false;
			}

			ignore_tooltip = true;

			return icon.drop_on_items(tab, eqslot, slot, equip);
		}

		return true;
	}

	Cursor::State UIItemInventory::send_cursor(bool pressed, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(pressed, cursorpos);

		if (dragged)
		{
			clear_tooltip();

			return dstate;
		}

		Point<int16_t> cursor_relative = cursorpos - position;

		if (!full_enabled && slider.isenabled())
		{
			Cursor::State sstate = slider.send_cursor(cursor_relative, pressed);

			if (sstate != Cursor::State::IDLE)
			{
				clear_tooltip();

				return sstate;
			}
		}

		int16_t slot = slot_by_position(cursor_relative);
		Icon *icon = get_icon(slot);
		bool is_icon = icon && is_visible(slot);

		if (is_icon)
		{
			if (pressed)
			{
				Point<int16_t> slotpos = get_slotpos(slot);
				icon->start_drag(cursor_relative - slotpos);
				UI::get().drag_icon(icon);

				clear_tooltip();

				return Cursor::State::GRABBING;
			} else if (!ignore_tooltip)
			{
				show_item(slot);

				return Cursor::State::CANGRAB;
			} else
			{
				ignore_tooltip = false;

				return Cursor::State::CANGRAB;
			}
		} else
		{
			clear_tooltip();

			return UIElement::send_cursor(pressed, cursorpos);
		}
	}

	void UIItemInventory::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed)
		{
			if (escape)
			{
				toggle_active();
			} else if (keycode == KeyAction::Id::TAB)
			{
				uint16_t next = (tab - InventoryType::Id::EQUIP + 1) % TABS;

				button_pressed(Buttons::BT_TAB0 + next);
			}
		}
	}

	UIElement::Type UIItemInventory::get_type() const
	{
		return TYPE;
	}

	void UIItemInventory::modify(InventoryType::Id type, int16_t slot, int8_t mode, int16_t arg)
	{
		if (slot <= 0)
			return;

		if (type == tab)
		{
			switch (mode)
			{
				case Inventory::Modification::ADD:
					update_slot(slot);
					break;
				case Inventory::Modification::CHANGECOUNT:
				case Inventory::Modification::ADDCOUNT:
					if (auto icon = get_icon(slot))
						icon->set_count(arg);

					break;
				case Inventory::Modification::SWAP:
					if (arg != slot)
					{
						update_slot(slot);
						update_slot(arg);
					}

					break;
				case Inventory::Modification::REMOVE:
					update_slot(slot);
					break;
			}
		}

		switch (mode)
		{
			case Inventory::Modification::ADD:
			case Inventory::Modification::ADDCOUNT:
				newtab = type;
				newslot = slot;
				break;
			case Inventory::Modification::CHANGECOUNT:
			case Inventory::Modification::SWAP:
			case Inventory::Modification::REMOVE:
				if (newslot == slot && newtab == type)
					clear_new();

				break;
		}
	}

	void UIItemInventory::set_sort(bool enabled)
	{
		sort_enabled = enabled;

		buttons[Buttons::BT_GATHER]->set_active(!sort_enabled);
		buttons[Buttons::BT_SORT]->set_active(sort_enabled);
	}

	void UIItemInventory::change_tab(InventoryType::Id type)
	{
		if (type >= InventoryType::Id::EQUIP && type <= InventoryType::Id::CASH)
			button_pressed(Buttons::BT_TAB0 + type - InventoryType::Id::EQUIP);
	}

	void UIItemInventory::clear_new()
	{
		newtab = InventoryType::Id::NONE;
		newslot = 0;
	}

	void UIItemInventory::toggle_active()
	{
		UIElement::toggle_active();

		if (!active)
		{
			clear_new();
			clear_tooltip();
		}
	}

	void UIItemInventory::remove_cursor()
	{
		UIDragElement::remove_cursor();

		slider.remove_cursor();
	}

	void UIItemInventory::show_item(int16_t slot)
	{
		if (tab == InventoryType::Id::EQUIP)
		{
			UI::get().show_equip(Tooltip::Parent::ITEMINVENTORY, slot);
		} else
		{
			int32_t item_id = inventory.get_item_id(tab, slot);
			auto item = inventory.get_item(tab, slot);
			auto pet = tab == InventoryType::Id::CASH ? inventory.get_pet(inventory.get_cash_id(tab, slot))
													  : Optional<const Pet>();

			if (item)
				UI::get().show_item(Tooltip::Parent::ITEMINVENTORY, item_id, item->get_expiration(), item->get_flags(),
									true);
			else if (pet)
				UI::get().show_item(Tooltip::Parent::ITEMINVENTORY, item_id, pet->get_date_dead(), 0, true,
									pet->get_remain_life());
			else
				UI::get().show_item(Tooltip::Parent::ITEMINVENTORY, item_id);
		}
	}

	void UIItemInventory::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::ITEMINVENTORY);
	}

	bool UIItemInventory::is_visible(int16_t slot) const
	{
		return slot >= first_slot() && slot <= last_slot();
	}

	int16_t UIItemInventory::first_slot() const
	{
		return full_enabled ? 1 : 1 + rows.at(tab) * COLUMNS;
	}

	int16_t UIItemInventory::last_slot() const
	{
		return first_slot() + (full_enabled ? MAXFULLSLOTS : MAXSLOTS) - 1;
	}

	bool UIItemInventory::can_change_equips() const
	{
		const Player &player = Stage::get().get_player();

		if (player.get_look().get_alerted())
		{
			UI::get().emplace<UIOk>(CATCH_BREATH, [](bool)
			{});
			return false;
		}

		if (player.is_dead())
		{
			UI::get().emplace<UIOk>(DEAD, [](bool)
			{});
			return false;
		}

		return true;
	}

	// Whether `stats` meet the equipment's requirements (FUN_004f2cee).
	static bool meets_equip_reqs(const CharStats &stats, const EquipData &equipdata)
	{
		// GMs wear anything (job digit 8 or 9).
		int32_t branch = stats.get_job().get_id() % 1000 / 100;

		if (branch == 8 || branch == 9)
			return true;

		const ItemData &itemdata = equipdata.get_itemdata();
		const int8_t reqGender = itemdata.get_gender();
		const bool female = stats.get_female();

		if ((reqGender == 0 && female) || (reqGender == 1 && !female))
			return false;

		// reqJob has a bit per branch from warriors on; 0 is anyone and -1
		// beginners only.
		int16_t reqJOB = equipdata.get_reqstat(MapleStat::Id::JOB);
		int32_t mask = branch ? 1 << (branch - 1) : 0;

		if (reqJOB != 0 && !(reqJOB == -1 && mask == 0) && !(reqJOB > 0 && (reqJOB & mask)))
			return false;

		return equipdata.get_reqstat(MapleStat::Id::LEVEL) <= stats.get_stat(MapleStat::Id::LEVEL)
			&& equipdata.get_reqstat(MapleStat::Id::DEX) <= stats.get_total(EquipStat::Id::DEX)
			&& equipdata.get_reqstat(MapleStat::Id::STR) <= stats.get_total(EquipStat::Id::STR)
			&& equipdata.get_reqstat(MapleStat::Id::LUK) <= stats.get_total(EquipStat::Id::LUK)
			&& equipdata.get_reqstat(MapleStat::Id::INT) <= stats.get_total(EquipStat::Id::INT)
			&& (equipdata.get_reqstat(MapleStat::Id::FAME) == 0
				|| equipdata.get_reqstat(MapleStat::Id::FAME) <= stats.get_honor());
	}

	bool UIItemInventory::meets_requirements(int32_t item_id)
	{
		return meets_equip_reqs(Stage::get().get_player().get_stats(), EquipData::get(item_id));
	}

	bool UIItemInventory::can_wear_equip(int16_t slot) const
	{
		const Player &player = Stage::get().get_player();
		const CharStats &stats = player.get_stats();

		if (!can_change_equips())
			return false;

		const int32_t item_id = inventory.get_item_id(InventoryType::Id::EQUIP, slot);
		const EquipData &equipdata = EquipData::get(item_id);

		// Only one of a Unique Equipped Item may be worn, not counting cash slots (FUN_004f2c5e).
		int16_t worn = inventory.find_item(InventoryType::Id::EQUIPPED, item_id);

		if (ItemData::get(item_id).get_properties().only_equip && worn > 0 && worn < Inventory::CASH_EQUIP)
		{
			UI::get().emplace<UIOk>("You can only be equipped with 1 Unique Equipped Item.", [](bool)
			{});
			return false;
		}

		if (meets_equip_reqs(stats, equipdata))
			return true;

		// v83 (FUN_004f1c2d) has one text for every unmet requirement.
		UI::get().emplace<UIOk>("Your stats are too low to equip this item.", [](bool)
		{});
		return false;
	}

	bool UIItemInventory::can_wear_pet_equip(int16_t slot, int16_t part) const
	{
		if (!can_change_equips())
			return false;

		int32_t item_id = inventory.get_item_id(InventoryType::Id::EQUIP, slot);
		int8_t owner = pets::equip_owner(part);
		pets::Wearer wearer = owner >= 0 ? pet_wearer(owner, item_id) : pets::Wearer();
		int16_t req_level = EquipData::get(item_id).get_reqstat(MapleStat::Id::LEVEL);

		if (const char *refusal = pets::refuse_equip(part, req_level, wearer))
		{
			UI::get().emplace<UIOk>(refusal, [](bool)
			{});
			return false;
		}

		return true;
	}

	pets::Wearer UIItemInventory::pet_wearer(uint8_t index, int32_t item_id) const
	{
		pets::Wearer wearer;
		Optional<PetLook> pet = Stage::get().get_player().get_pet(index);

		if (!pet)
			return wearer;

		int32_t kind = pet->get_itemid();
		const PetData &data = PetData::get(kind);

		wearer.out = true;
		// Clothes are drawn for the kinds they were made for.
		wearer.suits = item_id / 10000 == 180
					   && nl::nx::character["PetEquip"]["0" + std::to_string(item_id) + ".img"][std::to_string(kind)];
		wearer.name_tag = data.get_name_tag() != 0;
		wearer.chat_balloon = data.get_balloon() != 0;
		wearer.level = inventory.get_pet_level(pet->get_serial());

		return wearer;
	}

	void UIItemInventory::wear_pet_equip(int16_t slot, int32_t item_id) const
	{
		Player &player = Stage::get().get_player();
		std::array<pets::Wearer, 3> wearers;

		for (uint8_t i = 0; i < wearers.size(); i++)
			wearers[i] = pet_wearer(i, item_id);

		std::vector<pets::EquipChoice> choices = pets::equip_choices(item_id, wearers);

		auto wear = [slot, item_id](int16_t part)
		{
			auto window = UI::get().get_element<UIItemInventory>();

			// The item may have moved while the pet was chosen.
			if (!window || window->inventory.get_item_id(InventoryType::Id::EQUIP, slot) != item_id
				|| !window->can_wear_pet_equip(slot, part))
				return;

			put_on(slot, window->inventory.equip_position(slot, static_cast<EquipSlot::Id>(part)));
			Sound(Sound::Name::DRAGEND).play();
		};

		if (choices.size() == 1)
		{
			wear(choices[0].part);
		} else if (choices.size() > 1)
		{
			std::vector<UIPetChooser::Pet> shown;

			for (const pets::EquipChoice &choice : choices)
			{
				Optional<PetLook> look = player.get_pet(choice.pet);
				Optional<const Pet> pet = inventory.get_pet(look->get_serial());

				if (pet)
					shown.push_back({ look->get_itemid(), pet->get_name(), pet->get_level(),
									  static_cast<int16_t>(pet->get_closeness()) });
				else
					shown.push_back({ look->get_itemid(), "", 0, 0 });
			}

			UI::get().emplace<UIPetChooser>(UIPetChooser::EQUIP_CAPTION, std::move(shown),
											[wear, choices](size_t index)
			{
				wear(choices[index].part);
			});
		}
	}

	int16_t UIItemInventory::slot_by_position(Point<int16_t> cursorpos) const
	{
		Point<int16_t> offset = cursorpos - (full_enabled ? FULL_SLOT : SLOT);

		if (offset.x() < 0 || offset.y() < 0)
			return 0;

		int16_t page = 0;

		if (full_enabled)
		{
			page = offset.x() / PAGE_PITCH;
			offset.shift_x(-page * PAGE_PITCH);
		}

		int16_t column = offset.x() / SLOT_PITCH.x();
		int16_t row = offset.y() / SLOT_PITCH.y();

		if (page >= (full_enabled ? PAGES : 1) || column >= COLUMNS || row >= ROWS)
			return 0;

		int16_t slot = first_slot() + page * MAXSLOTS + row * COLUMNS + column;

		return is_visible(slot) ? slot : 0;
	}

	Point<int16_t> UIItemInventory::get_slotpos(int16_t slot) const
	{
		int16_t index = slot - first_slot();
		int16_t page = index / MAXSLOTS;
		int16_t column = index % COLUMNS;
		int16_t row = index % MAXSLOTS / COLUMNS;

		Point<int16_t> first = full_enabled ? FULL_SLOT : SLOT;

		return first + Point<int16_t>(page * PAGE_PITCH + column * SLOT_PITCH.x(), row * SLOT_PITCH.y());
	}

	Icon *UIItemInventory::get_icon(int16_t slot)
	{
		auto iter = icons.find(slot);

		if (iter != icons.end())
			return iter->second.get();
		else
			return nullptr;
	}

	void UIItemInventory::set_full(bool enabled)
	{
		full_enabled = enabled;

		dimension = (full_enabled ? full_backgrnd : backgrnd).get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), 20);

		int16_t close_x = dimension.x() - CLOSE_RIGHT;
		Point<int16_t> title_button = Point<int16_t>(close_x - TITLE_BUTTON_PITCH + 6, TITLE_BUTTON_Y);

		buttons[Buttons::BT_CLOSE]->set_position(Point<int16_t>(close_x, 6));
		buttons[Buttons::BT_GATHER]->set_position(title_button);
		buttons[Buttons::BT_SORT]->set_position(title_button);
		buttons[Buttons::BT_FULL]->set_position(title_button - Point<int16_t>(TITLE_BUTTON_PITCH, 0));
		buttons[Buttons::BT_SMALL]->set_position(title_button - Point<int16_t>(TITLE_BUTTON_PITCH, 0));
		buttons[Buttons::BT_COIN]->set_position(full_enabled ? FULL_COIN : COIN);

		buttons[Buttons::BT_FULL]->set_active(!full_enabled);
		buttons[Buttons::BT_SMALL]->set_active(full_enabled);

		set_sort(sort_enabled);
		load_icons();
	}

	void UIItemInventory::ItemIcon::drop_on_trade(int8_t slot) const
	{
		constexpr const char *trademessage = "How many will you trade?";

		Player &player = Stage::get().get_player();

		if (!player.can_request(ExclRequest::WAIT))
			return;

		if (!item_drop::tradeable(traits))
		{
			UI::get().emplace<UIOk>(item_drop::UNAVAILABLE, [](bool) {});

			return;
		}

		auto tab = static_cast<int8_t>(sourcetab);

		// v83 puts up one of anything that doesn't ask; the server takes
		// equipment and whole stacks of stars and bullets.
		if (!traits.expires && mini_room::asks_count(item_id, count))
		{
			auto onenter = [tab, source = source, slot](int32_t qty)
			{
				Stage::get().get_player().request(TradePutItemPacket(tab, source, qty, slot), ExclRequest::WAIT);
			};

			UI::get().emplace<UIEnterNumber>(trademessage, onenter, count, count);
		} else
		{
			player.request(TradePutItemPacket(tab, source, 1, slot), ExclRequest::WAIT);
		}
	}

	void UIItemInventory::ItemIcon::drop_on_shop() const
	{
		// v83 refuses these before asking anything (FUN_004f41b9).
		if (const char *refusal = item_drop::unsellable(traits))
		{
			UI::get().emplace<UIOk>(refusal, [](bool) {});

			return;
		}

		if (auto shop = UI::get().get_element<UIPersonalShop>())
			shop->sell(sourcetab, source, item_id, count);
	}

	void UIItemInventory::ItemIcon::drop_on_megaphone() const
	{
		// v83 takes any item here; the server decides (FUN_004f49b3).
		if (auto megaphone = UI::get().get_element<UIItemMegaphone>())
			megaphone->set_item(sourcetab, source);
	}

	void UIItemInventory::ItemIcon::set_count(int16_t c)
	{
		count = c;
	}

	Icon::IconType UIItemInventory::ItemIcon::get_type()
	{
		return Icon::IconType::ITEM;
	}

	UIItemInventory::ItemIcon::ItemIcon(const UIItemInventory &parent, InventoryType::Id st, EquipSlot::Id eqs,
										int16_t s, int32_t iid, int16_t c, item_drop::Traits t) : parent(parent)
	{
		sourcetab = st;
		eqsource = eqs;
		source = s;
		item_id = iid;
		count = c;
		traits = t;
	}

	void UIItemInventory::ItemIcon::drop_on_stage() const
	{
		constexpr const char *dropmessage = "How many will you drop?";
		constexpr const char *cashmessage = "You can't drop this item.";

		const Player &player = Stage::get().get_player();

		if (player.get_look().get_alerted())
		{
			UI::get().emplace<UIOk>(CATCH_BREATH, [](bool)
			{});

			return;
		}

		if (player.is_dead())
		{
			UI::get().emplace<UIOk>(DEAD, [](bool)
			{});

			return;
		}

		if (item_drop::kept(item_id))
			return;

		if (item_drop::refused(item_id, traits.cash))
		{
			UI::get().emplace<UIOk>(cashmessage, [](bool)
			{});

			return;
		}

		auto drop = [tab = sourcetab, slot = source, id = item_id, max = count, expires = traits.expires, dropmessage]()
		{
			if (item_drop::asks_count(id, max, expires))
			{
				auto onenter = [tab, slot](int32_t qty)
				{
					send_drop(tab, slot, qty);
				};

				UI::get().emplace<UIEnterNumber>(dropmessage, onenter, max, max);
			} else
			{
				send_drop(tab, slot, 1);
			}
		};

		confirm(item_drop::steps(item_id, traits), 0, drop);
	}

	void UIItemInventory::ItemIcon::drop_on_equips(EquipSlot::Id eqslot) const
	{
		switch (sourcetab)
		{
			case InventoryType::Id::EQUIP:
				if (std::optional<std::array<int16_t, 3>> parts = pets::equip_parts(item_id))
				{
					// Pet equipment goes on the cell it was dropped on, of
					// the pet whose tab is open (FUN_004f1b67).
					if (std::find(parts->begin(), parts->end(), eqslot) != parts->end()
						&& parent.can_wear_pet_equip(source, eqslot))
					{
						put_on(source, Stage::get().get_player().get_inventory().equip_position(source, eqslot));
					}
				} else if (eqsource == eqslot)
				{
					if (parent.can_wear_equip(source))
						put_on(source, Stage::get().get_player().get_inventory().equip_position(source, eqslot));
				}

				Sound(Sound::Name::DRAGEND).play();

				break;
			case InventoryType::Id::USE:
			{
				// v83 FUN_004f4bb7: a scroll dropped on a worn equip. HeavenClient
				// has no sealed items, so v83's notices about those don't apply.
				Player &player = Stage::get().get_player();

				if (player.get_look().get_alerted())
				{
					UI::get().emplace<UIOk>(CATCH_BREATH, [](bool)
					{});
					break;
				}

				if (player.is_dead())
				{
					UI::get().emplace<UIOk>(DEAD, [](bool)
					{});
					break;
				}

				if (!scroll_rules::is_scroll(item_id) || !player.can_request(ExclRequest::WAIT))
					break;

				const Inventory &inventory = player.get_inventory();

				// The scroll goes on the item shown there. Also refuses an empty slot.
				int16_t target = inventory.shown_equip(eqslot);

				if (!scroll_rules::fits(item_id, inventory.get_item_id(InventoryType::Id::EQUIPPED, target)))
					break;

				auto send = [slot = source, target](bool whitescroll)
				{
					Stage::get().get_player().request(ScrollEquipPacket(slot, target, whitescroll), ExclRequest::WAIT);
				};

				// Either answer scrolls; only Yes uses the White Scroll.
				if (scroll_rules::takes_white_scroll(item_id) &&
					inventory.find_item(InventoryType::Id::USE, scroll_rules::WHITE_SCROLL) != 0)
					UI::get().emplace<UIYesNo>("Would you like to use the White Scroll?", send);
				else
					send(false);

				break;
			}
		}
	}

	bool UIItemInventory::ItemIcon::drop_on_items(InventoryType::Id tab, EquipSlot::Id, int16_t slot, bool) const
	{
		if (tab != sourcetab || slot == source)
			return true;

		Stage::get().get_player().request(MoveItemPacket(tab, source, slot, 1), ExclRequest::WAIT);

		return true;
	}

	void UIItemInventory::ItemIcon::drop_on_bindings(Point<int16_t> cursorposition, bool remove) const
	{
		Keyboard::Mapping mapping = get_mapping();

		if (mapping.type == KeyType::Id::NONE)
			return;

		auto keyconfig = UI::get().get_element<UIKeyConfig>();

		if (remove)
			keyconfig->unstage_mapping(mapping);
		else
			keyconfig->stage_mapping(cursorposition, mapping);
	}

	// Only use and set-up items go on keys.
	Keyboard::Mapping UIItemInventory::ItemIcon::get_mapping() const
	{
		if (sourcetab == InventoryType::Id::USE || sourcetab == InventoryType::Id::SETUP)
			return Keyboard::Mapping(KeyType::ITEM, item_id);

		return {};
	}
}
