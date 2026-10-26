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
#include "UICashBuy.h"
#include "UICashGift.h"
#include "UINameChange.h"
#include "UICashShop.h"
#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Character/Marriage.h"
#include "../../Character/MountRules.h"
#include "../../Data/ItemData.h"
#include "../../Data/NameChange.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		using namespace cash_catalog;

		// Dialog coordinates, measured on CSNotice/0 and period footage.
		const Point<int16_t> ICON_POS = Point<int16_t>(21, 58);
		const Point<int16_t> NAME_POS = Point<int16_t>(59, 25);
		const Point<int16_t> PRICE_POS = Point<int16_t>(59, 42);
		const Point<int16_t> QUESTION_POS = Point<int16_t>(24, 69);
		// A question in words takes the icon's place too.
		const Point<int16_t> WORDS_POS = Point<int16_t>(21, 25);
		// The checkboxes sit left of the payment labels baked into the art.
		const Point<int16_t> CHECKBOX_POS = Point<int16_t>(28, 102);
		constexpr int16_t CHECKBOX_STEP = 14;
		constexpr int16_t CHECKBOX_SIZE = 12;
		// The labels end near x 193; a click on one checks it too.
		constexpr int16_t OPTION_WIDTH = 166;

		// Packages are items 910xxxx, and slot expansions 911x000: 9110000
		// for the storage, then one for each bag from Equip to Etc.
		bool is_package(int32_t item_id)
		{
			return item_id / 10000 == 910;
		}

		bool is_expansion(int32_t item_id)
		{
			return item_id / 10000 == 911;
		}

		InventoryType::Id expanded(int32_t item_id)
		{
			return static_cast<InventoryType::Id>(item_id / 1000 % 10);
		}

		// StringPool 0x217, 0x1eb, 0x1ec and 0x11b0.
		constexpr const char* NOT_ENOUGH_CASH = "You don't have enough cash.";
		constexpr const char* BAG_FULL = "The requested number of slots\\ncan't be added.";
		constexpr const char* STORAGE_FULL = "The storage slots can't be extended.";
		// StringPool 0x1ed.
		constexpr const char* NO_MORE_CHARACTERS = "You may not add any more character slots.";

		// StringPool 0xb94 and 0x1f2.
		constexpr const char* ALREADY_HAVE = "You may not purchase the item\\nsince you already have it.";
		constexpr const char* BUY_AGAIN = "You already have the item(s) selected. Are\\nyou sure you want to buy the item(s) again?";

		// StringPool 0x1f9.
		constexpr const char* NO_RIDING = "This is an item that cannot be used\\nif you do not have the Monster\\nRiding skill. Would you\\nstill like to purchase it?";

		std::string advancement_question(int32_t slots)
		{
			return "You can expand " + std::to_string(slots) + " slots when\\nyou change occupations.\\nWould you really like to\\nexpand now even at a cost?";
		}

		// The question a character slot or a slot button asks in place of
		// the item, or nothing. v83 names the bags from a table of its own
		// (StringPool 0xa, 0x15dc, 0xb, 0x15a2 and 0x15b5).
		constexpr const char* BAG_NAMES[InventoryType::Id::EQUIPPED] = { "", "Eqp", "Use", "Setup", "Etc", "Pet" };

		std::string slots_question(const Commodity& commodity)
		{
			std::string price = std::to_string(commodity.price);

			if (is_character_slot(commodity.item_id))
				return "Add a character slot (" + price + " NX)\\nAre you sure you want to add another\\ncharacter slot?";

			// The days are the item's addDay, not the commodity's period
			// (46c950).
			if (is_slot_extender(commodity.item_id))
			{
				std::string id = "0" + std::to_string(commodity.item_id);
				int32_t days = static_cast<int32_t>(nl::nx::item["Cash"]["0555.img"][id]["info"]["addDay"].get_integer());

				return slot_extender_question(ItemData::get(commodity.item_id).get_name(), commodity.price, days);
			}

			if (!is_expansion(commodity.item_id) || commodity.sn != 0)
				return "";

			InventoryType::Id bag = expanded(commodity.item_id);

			if (bag == InventoryType::Id::NONE)
				return "Adding slots to the storage (" + price + " NX)\\nAre you sure you want to add slots\\nto your storage?";

			return std::string(BAG_NAMES[bag]) + " adding inventory slots. (" + price + " NX)\\nConfirm inventory slot purchase?";
		}

		// A slot button adds four slots, a commodity its item's delta.
		int32_t added_slots(const Commodity& commodity)
		{
			if (commodity.sn == 0)
				return EXPANSION_SLOTS;

			return static_cast<int32_t>(nl::nx::item["Special"]["0911.img"][std::to_string(commodity.item_id)]["delta"].get_integer());
		}

		int32_t storage_slots()
		{
			auto cashshop = UI::get().get_element<UICashShop>();

			return cashshop ? cashshop->get_storage_slots() : 0;
		}

		int16_t character_slots()
		{
			auto cashshop = UI::get().get_element<UICashShop>();

			return cashshop ? cashshop->get_character_slots() : 0;
		}

		// Whether the bag or the storage an expansion widens has room for it.
		bool has_room(const Commodity& commodity)
		{
			InventoryType::Id bag = expanded(commodity.item_id);
			int32_t added = added_slots(commodity);

			if (bag == InventoryType::Id::NONE)
				return storage_slots() + added <= MAX_STORAGE_SLOTS;

			return Stage::get().get_player().get_inventory().get_slotmax(bag) + added <= MAX_BAG_SLOTS;
		}

		void notice(const char* text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}

		// Whether the character already has the item, or one of a
		// package's (v83 4729b5 with flag 0, and 46e121).
		bool owned(const Commodity& commodity)
		{
			auto cashshop = UI::get().get_element<UICashShop>();

			if (!cashshop)
				return false;

			if (is_package(commodity.item_id))
				return cashshop->owns_package(commodity.item_id);

			return cashshop->owns(commodity.item_id, true);
		}
	}

	void UICashBuy::open(const Commodity& commodity, int32_t credit, int32_t points, int32_t prepaid)
	{
		auto buy = [=]() { UI::get().emplace<UICashBuy>(commodity, credit, points, prepaid); };

		// v83 47326d: Name Change asks for the birthday code, and is paid
		// with NX Prepaid alone.
		if (commodity.item_id == name_change::ITEM)
		{
			UI::get().emplace<UINameChangeBirthday>(commodity.sn, commodity.price, prepaid);

			return;
		}

		// v83 46c6f8: a character slot, while the account has fewer than 15.
		if (is_character_slot(commodity.item_id))
		{
			if (!affordable(commodity.price, credit, points, prepaid))
				return notice(NOT_ENOUGH_CASH);

			if (character_slots() >= MAX_CHARACTER_SLOTS)
				return notice(NO_MORE_CHARACTERS);

			return buy();
		}

		// v83 46c8e1: a slot extender, paid with NX Credit or Maple Points.
		if (is_slot_extender(commodity.item_id))
		{
			if (commodity.price > 0 && !affordable(commodity.price, credit, points, 0))
				return notice(NOT_ENOUGH_CASH);

			return buy();
		}

		// v83 4731a9: a crush or friendship ring is bought for two.
		if (marriage::is_pair_ring(commodity.item_id))
		{
			UI::get().emplace<UICashGift>(commodity, credit, points, prepaid);

			return;
		}

		if (!is_expansion(commodity.item_id))
		{
			// v83 46dadd: some balance must pay, NX Prepaid alone for some
			// items (0x251).
			if (commodity.price > 0 && !affordable(commodity.price, credit, points, prepaid))
				return notice(NOT_ENOUGH_CASH);

			if (prepaid_only(commodity.item_id) && prepaid < commodity.price)
				return notice(name_change::NO_PREPAID);

			// v83 46d211: a mount or saddle, for a job without Monster Rider.
			Player& player = Stage::get().get_player();
			int32_t rider = mount::rider_skill(player.get_stats().get_job().get_id());
			int32_t prefix = commodity.item_id / 10000;

			auto ask_riding = [buy, prefix, rider]()
			{
				if ((prefix == 190 || prefix == 191) && Stage::get().get_player().get_skills().get_level(rider) <= 0)
					UI::get().emplace<UIYesNo>(NO_RIDING, [buy](bool yes) { if (yes) buy(); });
				else
					buy();
			};

			// An item the character has: a free one can't be bought again,
			// another asks first.
			if (!owned(commodity))
				ask_riding();
			else if (commodity.price <= 0)
				notice(ALREADY_HAVE);
			else
				UI::get().emplace<UIYesNo>(BUY_AGAIN, [ask_riding](bool yes) { if (yes) ask_riding(); });

			return;
		}

		// v83 46c036, 46c55b and 470653; the dialog checks the bags' room.
		if (commodity.price > 0 && !affordable(commodity.price, credit, points, prepaid))
			return notice(NOT_ENOUGH_CASH);

		InventoryType::Id bag = expanded(commodity.item_id);

		if (bag == InventoryType::Id::NONE)
		{
			if (commodity.sn == 0 && !has_room(commodity))
				return notice(STORAGE_FULL);

			return buy();
		}

		const Player& player = Stage::get().get_player();
		int32_t job = player.get_stats().get_job().get_id();
		int32_t slots = player.get_inventory().get_slotmax(bag);

		if (int32_t later = advancement_slots(job, bag, slots, added_slots(commodity)))
			UI::get().emplace<UIYesNo>(advancement_question(later), [buy](bool yes) { if (yes) buy(); });
		else
			buy();
	}

	// v83's ring dialogs offer NX Credit and NX Prepaid (46e08e with flags
	// 1 and 4), so the Maple Points' option is never checked; the items
	// NX Prepaid alone pays for offer it alone (46dc4d).
	UICashBuy::UICashBuy(const Commodity& commodity, int32_t credit, int32_t points, int32_t prepaid, std::optional<RingOrder> ring) : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), sn(commodity.sn), item_id(commodity.item_id), ring(std::move(ring))
	{
		if (this->ring)
			offered = { false, true, true };
		else if (prepaid_only(item_id))
			offered = { false, true, false };
		else
			offered = payment_options(commodity.price, credit, points, prepaid);

		// Checked first: an offered balance that pays.
		std::array<bool, NUM_PAYMENTS> pays = payment_options(commodity.price, credit, this->ring ? 0 : points, prepaid);

		for (size_t i = 0; i < NUM_PAYMENTS; i++)
			pays[i] = pays[i] && offered[i];

		payment = default_payment(pays);

		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		int16_t height = 0;

		for (nl::node part : NxFiles::ui_classic["CashShop.img"]["CSNotice"]["0"])
		{
			parts.emplace_back(part);
			height += parts.back().height();
		}

		checkboxes[0] = Basic["CheckBox"]["0"];
		checkboxes[1] = Basic["CheckBox"]["1"];

		dimension = Point<int16_t>(parts.front().width(), height);
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2, (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(Basic["BtOK2"], Point<int16_t>(163, 158));
		buttons[Buttons::BT_CANCEL] = std::make_unique<MapleButton>(Basic["BtCancel2"], Point<int16_t>(213, 158));

		std::string price_text = std::to_string(commodity.price) + " NX Cash";

		if (commodity.count > 1)
			price_text += "(" + std::to_string(commodity.count) + ")";

		if (commodity.period > 0)
			price_text += " / " + std::to_string(commodity.period) + " day";

		name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, ItemData::get(item_id).get_name(), 200);
		price = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, price_text);
		question = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "Are you sure you want to buy this item?");

		// StringPool 0x1e9 asks for a character slot instead, and 0x1e7 and
		// 0x1e8 for a slot button's slots (v83 46c6f8, 46c036 and 46c55b),
		// in a dialog without the item (46c4d5).
		if (std::string text = slots_question(commodity); !text.empty())
		{
			words = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, text, dimension.x() - 2 * WORDS_POS.x());
			name = Text();
			price = Text();
			question = Text();
		}
	}

	UICashBuy::UICashBuy(const std::vector<int32_t>& avatar, int32_t total, const std::array<bool, NUM_PAYMENTS>& offered) : UICashBuy(Commodity{ 0, 0, total }, 0, 0, 0)
	{
		this->avatar = avatar;
		this->offered = offered;
		payment = default_payment(offered);

		words = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, avatar_question(total), dimension.x() - 2 * WORDS_POS.x());
		name = Text();
		price = Text();
		question = Text();
	}

	void UICashBuy::draw(float inter) const
	{
		Point<int16_t> pos = position;

		for (const Texture& part : parts)
		{
			part.draw(pos);
			pos.shift_y(part.height());
		}

		if (words.empty())
			ItemData::get(item_id).get_icon(false).draw(position + ICON_POS);

		words.draw(position + WORDS_POS);
		name.draw(position + NAME_POS);
		price.draw(position + PRICE_POS);
		question.draw(position + QUESTION_POS);

		for (size_t i = 0; i < NUM_PAYMENTS; i++)
			checkboxes[i == payment].draw(position + CHECKBOX_POS + Point<int16_t>(0, CHECKBOX_STEP * i));

		UIElement::draw(inter);
	}

	Cursor::State UICashBuy::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> cursor = cursorpos - position - CHECKBOX_POS;

		if (cursor.x() >= 0 && cursor.x() < OPTION_WIDTH && cursor.y() >= 0 && cursor.y() < CHECKBOX_STEP * static_cast<int16_t>(NUM_PAYMENTS) && cursor.y() % CHECKBOX_STEP < CHECKBOX_SIZE)
		{
			size_t option = cursor.y() / CHECKBOX_STEP;

			if (!offered[option])
				return UIElement::send_cursor(clicked, cursorpos);

			if (clicked)
				payment = option;

			return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UICashBuy::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			deactivate();
		else if (keycode == KeyAction::Id::RETURN)
			buy();
	}

	UIElement::Type UICashBuy::get_type() const
	{
		return TYPE;
	}

	Button::State UICashBuy::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::BT_OK)
			buy();
		else
			deactivate();

		return Button::State::NORMAL;
	}

	void UICashBuy::buy()
	{
		CashType cash_type = PAYMENTS[payment];
		auto cashshop = UI::get().get_element<UICashShop>();

		if (!avatar.empty())
		{
			if (cashshop)
				cashshop->buy_avatar(avatar, cash_type);

			deactivate();

			return;
		}

		if (is_expansion(item_id) && !has_room({ sn, item_id }))
		{
			deactivate();
			notice(expanded(item_id) == InventoryType::Id::NONE ? STORAGE_FULL : BAG_FULL);

			return;
		}

		if (cashshop && cashshop->request())
		{
			if (ring)
				BuyCoupleRingPacket(ring->friendship, ring->birthday, cash_type, sn, ring->to, ring->message).dispatch();
			else if (is_package(item_id))
				BuyCashPackagePacket(cash_type, sn).dispatch();
			else if (is_character_slot(item_id))
				BuyCharacterSlotPacket(cash_type, sn).dispatch();
			else if (is_slot_extender(item_id))
				BuySlotExtenderPacket(cash_type, sn).dispatch();
			else if (is_expansion(item_id) && sn == 0)
				ExpandSlotsPacket(cash_type, expanded(item_id)).dispatch();
			else if (is_expansion(item_id))
				ExpandSlotsPacket(cash_type, expanded(item_id), sn).dispatch();
			else
				BuyCashItemPacket(cash_type, sn).dispatch();
		}

		deactivate();
	}
}
