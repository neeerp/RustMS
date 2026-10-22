#include "UIKarmaScissors.h"

#include "UIChatBar.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Character/Inventory/ItemExpiry.h"
#include "../../Character/Inventory/ItemProperties.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// From the window's top left (0x830c6c, 0x830e36, 0x830e9e). The icon's origin is its bottom left.
		constexpr Point<int16_t> ICON = Point<int16_t>(74, 126);
		constexpr Rectangle<int16_t> ICON_HOVER = Rectangle<int16_t>(75, 102, 100, 127);
		// As the Item Guard's: not checked against v83.
		constexpr int16_t DRAG_HEIGHT = 25;

		// StringPool 0x129.
		constexpr const char *REQUEST_BUSY = "Other requests are in process.\\nPlease try again in a little bit.";

		void notice(const std::string &text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}
	}

	UIKarmaScissors::UIKarmaScissors(int16_t s, int32_t id)
			: UIElement(Point<int16_t>(), Point<int16_t>()), slot(s), item_id(id),
			  target_tab(InventoryType::Id::NONE), target_slot(0), target_id(0), dragged(false)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node backgrnd = NxFiles::ui_classic["UIWindow.img"]["KarmaScissors"]["backgrnd"];

		Texture background = backgrnd;
		sprites.emplace_back(backgrnd);

		buttons[Buttons::OK] = std::make_unique<MapleButton>(basic["BtOK2"], Point<int16_t>(72, 153));
		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(basic["BtClose"], Point<int16_t>(158, 8));

		// Centred, as the Item Guard's.
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);
	}

	void UIKarmaScissors::draw(float alpha) const
	{
		UIElement::draw(alpha);

		if (target_tab != InventoryType::Id::NONE)
		{
			int32_t id = Stage::get().get_player().get_inventory().get_item_id(target_tab, target_slot);

			if (id)
				ItemData::get(id).get_icon(false).draw(position + ICON);
		}
	}

	void UIKarmaScissors::remove_cursor()
	{
		UIElement::remove_cursor();

		dragged = false;
		clear_tooltip();
	}

	Cursor::State UIKarmaScissors::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (dragged)
		{
			if (clicked)
			{
				position = cursorpos - drag_from;

				return Cursor::State::CLICKING;
			}

			dragged = false;
		}

		Point<int16_t> cursor = cursorpos - position;
		const Inventory &inventory = Stage::get().get_player().get_inventory();
		int32_t id = target_tab == InventoryType::Id::NONE ? 0 : inventory.get_item_id(target_tab, target_slot);

		if (id && ICON_HOVER.contains(cursor))
		{
			if (target_tab == InventoryType::Id::EQUIP)
				UI::get().show_equip(Tooltip::Parent::KARMASCISSORS, target_slot);
			else if (auto item = inventory.get_item(target_tab, target_slot))
				UI::get().show_item(Tooltip::Parent::KARMASCISSORS, id, item->get_expiration(), item->get_flags(),
									true);
		}
		else
		{
			clear_tooltip();
		}

		Cursor::State ret = UIElement::send_cursor(clicked, cursorpos);

		if (clicked && cursor.y() >= 0 && cursor.y() < DRAG_HEIGHT && !is_holding_button())
		{
			drag_from = cursor;
			dragged = true;
		}

		return ret;
	}

	void UIKarmaScissors::send_key(int32_t keycode, bool pressed, bool escape)
	{
		// As the Item Guard's window, whose key handler it shares (0x4edd21).
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			close();
	}

	bool UIKarmaScissors::send_icon(const Icon &icon, Point<int16_t>)
	{
		// 0x4f4abe: only from the Item window. A refused drop keeps the last.
		auto [tab, s] = icon.get_source();

		if (tab == InventoryType::Id::NONE || tab == InventoryType::Id::EQUIPPED)
			return true;

		auto target = target_at(tab, s);

		if (!target)
			return true;

		karma_scissors::Drop drop = karma_scissors::check(*target);

		if (drop != karma_scissors::Drop::ACCEPTED)
		{
			notice(karma_scissors::refusal(drop));

			return true;
		}

		target_tab = tab;
		target_slot = s;
		target_id = target->item_id;

		return true;
	}

	UIElement::Type UIKarmaScissors::get_type() const
	{
		return TYPE;
	}

	Button::State UIKarmaScissors::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::OK)
			ok();
		else
			close();

		return Button::State::NORMAL;
	}

	std::optional<karma_scissors::Target> UIKarmaScissors::target_at(InventoryType::Id tab, int16_t s)
	{
		const Inventory &inventory = Stage::get().get_player().get_inventory();
		int32_t id = inventory.get_item_id(tab, s);

		if (!id)
			return std::nullopt;

		karma_scissors::Target target;
		target.item_id = id;
		target.trade_available = ItemData::get(id).get_properties().trade_available;

		if (auto equip = inventory.get_equip(tab, s))
		{
			target.sealed = equip->get_flags() & item_expiry::SEALED;
			target.karma = equip->get_flags() & item_properties::EQUIP_KARMA;
			target.bound = equip->get_flags() & item_properties::EQUIP_UNTRADEABLE;
		}
		else if (auto item = inventory.get_item(tab, s))
		{
			target.sealed = item->get_flags() & item_expiry::SEALED;
			target.karma = item->get_flags() & item_properties::BUNDLE_KARMA;
		}

		return target;
	}

	void UIKarmaScissors::ok()
	{
		// 0x830fb5: nothing without an item. HeavenClient also checks the item in its place again, as it may have
		// changed since it was dropped in.
		auto target = target_tab == InventoryType::Id::NONE ? std::nullopt : target_at(target_tab, target_slot);

		if (!target || target->item_id != target_id)
			return;

		karma_scissors::Drop drop = karma_scissors::check(*target);

		if (drop != karma_scissors::Drop::ACCEPTED)
		{
			notice(karma_scissors::refusal(drop));

			return;
		}

		Player &player = Stage::get().get_player();

		if (player.get_inventory().get_item_id(InventoryType::Id::CASH, slot) != item_id)
			return;

		if (!player.can_request(ExclRequest::WAIT))
		{
			notice(REQUEST_BUSY);

			return;
		}

		if (!player.request(UseOnItemPacket(slot, item_id, target_tab, target_slot), ExclRequest::WAIT))
			return;

		Sound(nl::nx::sound["Game.img"]["UseShopItem"]).play();

		// Printed as it is sent, before any answer.
		if (auto chatbar = UI::get().get_element<UIChatBar>())
			chatbar->send_chatline(karma_scissors::USED, UIChatBar::LineType::SKILL);

		close();
	}

	void UIKarmaScissors::close()
	{
		clear_tooltip();
		deactivate();
	}

	void UIKarmaScissors::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::KARMASCISSORS);
	}
}
