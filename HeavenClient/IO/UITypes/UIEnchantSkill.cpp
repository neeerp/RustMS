#include "UIEnchantSkill.h"

#include "UIChatBar.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Audio/Audio.h"
#include "../../Character/Inventory/ItemDrop.h"
#include "../../Character/Inventory/ScrollRules.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// From the window's top left (OnCreate 0x7f5c9b, draw FUN_007f6196).
		// The equip's icon has its origin at the bottom left, and its tooltip
		// shows over the box around it (0x7f637f).
		constexpr Point<int16_t> SAY = Point<int16_t>(46, 26);
		constexpr Point<int16_t> WHITE_LABEL = Point<int16_t>(30, 150);
		constexpr Point<int16_t> WHITE_BOX = Point<int16_t>(10, 153);
		constexpr Point<int16_t> WHITE_AREA = Point<int16_t>(150, 15);
		constexpr Point<int16_t> ICON = Point<int16_t>(74, 126);
		constexpr Rectangle<int16_t> ICON_HOVER = Rectangle<int16_t>(75, 102, 100, 127);
		// The effect covers the magic circle (I: its place isn't traced).
		constexpr Point<int16_t> EFFECT = Point<int16_t>(3, 62);
		// The effect's frames have no delay; v83 plays them 60 ms apart (I).
		constexpr uint16_t EFFECT_DELAY = 60;

		// As for a scroll dropped on a worn equip (FUN_004f4bb7's prologue).
		constexpr const char *CATCH_BREATH = "You can't do it right now.\\nCatch your breath and then try again.";
		constexpr const char *DEAD = "You can't do it once you're dead.";

		void notice(const std::string &text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}

		int32_t item_at(InventoryType::Id tab, int16_t slot)
		{
			return slot ? Stage::get().get_player().get_inventory().get_item_id(tab, slot) : 0;
		}
	}

	UIEnchantSkill::UIEnchantSkill()
			: UIElement(Point<int16_t>(), Point<int16_t>()), equip_slot(0), locked(false), white(false), white_sent(false),
			  success(false), cursed(false), playing(false)
	{
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["EnchantSkill"];

		Texture background = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);

		for (size_t i = 0; i < say.size(); i++)
			say[i] = src["say" + std::to_string(i)];

		box[false] = basic["CheckBox"]["0"];
		box[true] = basic["CheckBox"]["1"];

		std::vector<Frame> frames;

		for (int32_t i = 0; src["Effect"][std::to_string(i)]; i++)
			frames.emplace_back(src["Effect"][std::to_string(i)], EFFECT_DELAY, 255, 255);

		effect = Animation(frames);

		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(basic["BtCancel2"], Point<int16_t>(65, 180));
		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(basic["BtClose"], Point<int16_t>(158, 8));
		buttons[Buttons::WHITE] = std::make_unique<AreaButton>(WHITE_BOX, WHITE_AREA);
		buttons[Buttons::WHITE]->set_active(holds_white_scroll());

		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);
	}

	void UIEnchantSkill::draw(float alpha) const
	{
		UIElement::draw(alpha);

		int32_t equip = item_at(InventoryType::Id::EQUIP, equip_slot);

		say[equip ? 1 : 0].draw(position + SAY);

		// The box is there only for a player with a White Scroll (FUN_007f6f0a).
		if (buttons.at(Buttons::WHITE)->is_active())
		{
			say[2].draw(position + WHITE_LABEL);
			box[white].draw(position + WHITE_BOX);
		}

		if (equip)
			ItemData::get(equip).get_icon(false).draw(position + ICON);

		if (playing)
			effect.draw(position + EFFECT, alpha);
	}

	void UIEnchantSkill::update()
	{
		UIElement::update();

		// v83 sets the box again once the effect is over; until then it stays.
		if (!playing)
			buttons[Buttons::WHITE]->set_active(holds_white_scroll());

		if (playing && effect.update())
		{
			playing = false;
			finish();
		}
	}

	void UIEnchantSkill::remove_cursor()
	{
		UIElement::remove_cursor();

		UI::get().clear_tooltip(Tooltip::Parent::ENCHANTSKILL);
	}

	Cursor::State UIEnchantSkill::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (item_at(InventoryType::Id::EQUIP, equip_slot) && ICON_HOVER.contains(cursorpos - position))
			UI::get().show_equip(Tooltip::Parent::ENCHANTSKILL, equip_slot);
		else
			UI::get().clear_tooltip(Tooltip::Parent::ENCHANTSKILL);

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIEnchantSkill::send_key(int32_t, bool pressed, bool escape)
	{
		// v83 adds no keys of its own; Esc closes it as Cancel would.
		if (pressed && escape && !locked)
			close();
	}

	bool UIEnchantSkill::send_icon(const Icon &icon, Point<int16_t>)
	{
		if (locked)
			return true;

		auto [tab, slot] = icon.get_source();
		int32_t id = tab == InventoryType::Id::NONE ? 0 : item_at(tab, slot);

		// FUN_004ef140: an equip from the Equip tab is the one to scroll
		// (SetItem, FUN_007f63fa), a scroll from the Use tab goes on it.
		if (tab == InventoryType::Id::EQUIP && id / 1000000 == 1)
		{
			if (!scroll_rules::may_enchant(id, ItemData::get(id).is_cashitem()))
			{
				notice(scroll_rules::CASH_REFUSED);

				return true;
			}

			Sound(Sound::Name::DRAGEND).play();
			equip_slot = slot;
		}
		else if (tab == InventoryType::Id::USE)
		{
			scroll(slot);
		}

		return true;
	}

	UIElement::Type UIEnchantSkill::get_type() const
	{
		return TYPE;
	}

	void UIEnchantSkill::receive(int8_t result, bool c)
	{
		if (!locked || playing)
			return;

		if (result == -1)
		{
			notice(scroll_rules::ENCHANT_REFUSED);
			lock(false);

			return;
		}

		success = result != 0;
		cursed = c;
		playing = true;
		effect.play(Animation::End::FIRST);
		Sound(nl::nx::sound["UI.img"]["Enchant"]).play();
	}

	Button::State UIEnchantSkill::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::WHITE)
			white = !white;
		else if (!locked)
			close();

		return Button::State::NORMAL;
	}

	void UIEnchantSkill::scroll(int16_t slot)
	{
		// FUN_004f4bb7 with legendary 1.
		Player &player = Stage::get().get_player();
		const Inventory &inventory = player.get_inventory();
		int32_t scroll_id = inventory.get_item_id(InventoryType::Id::USE, slot);
		int32_t equip = item_at(InventoryType::Id::EQUIP, equip_slot);

		if (player.get_look().get_alerted())
		{
			notice(CATCH_BREATH);

			return;
		}

		if (player.is_dead())
		{
			notice(DEAD);

			return;
		}

		if (inventory.is_sealed(InventoryType::Id::USE, slot))
		{
			notice(item_drop::SEALED);

			return;
		}

		// Without an equip, or with one the scroll doesn't fit, nothing is
		// said; the server checks the slots left.
		if (!equip || !scroll_rules::fits(scroll_id, equip) || !player.can_request(ExclRequest::WAIT))
			return;

		// The box answers v83's question about the White Scroll, which some
		// scrolls never use.
		bool whitescroll = white && holds_white_scroll() && scroll_rules::takes_white_scroll(scroll_id);
		int16_t target = equip_slot;

		auto send = [slot, target, whitescroll]()
		{
			auto window = UI::get().get_element<UIEnchantSkill>();

			if (!window || !window->is_active() || window->locked || window->equip_slot != target)
				return;

			if (Stage::get().get_player().request(LegendaryScrollPacket(slot, target, whitescroll), ExclRequest::WAIT))
			{
				window->white_sent = whitescroll;
				window->lock(true);
			}
		};

		// A sealed equip asks first (0x4f4e41); No stops there.
		if (inventory.is_sealed(InventoryType::Id::EQUIP, equip_slot))
			UI::get().emplace<UIYesNo>(scroll_rules::SEALED_TARGET, [send](bool yes)
			{
				if (yes)
					send();
			});
		else
			send();
	}

	void UIEnchantSkill::finish()
	{
		// The line goes by the box as it is now, not by the answer's byte, but never tells of a White Scroll
		// that didn't go (the last one, gone with the box, or one the scroll doesn't use).
		if (auto chatbar = UI::get().get_element<UIChatBar>())
			chatbar->send_chatline(scroll_rules::result_text(success, cursed, white && white_sent),
								   UIChatBar::LineType::SKILL);

		Sound(success ? Sound::Name::ENCHANTSUCCESS : Sound::Name::ENCHANTFAILURE).play();
		Stage::get().get_player().show_effect_id(success ? CharEffect::Id::SCROLL_SUCCESS
														 : CharEffect::Id::SCROLL_FAILURE);

		// A destroyed equip closes the window; otherwise it stays for another scroll.
		if (cursed)
			close();
		else
			lock(false);
	}

	void UIEnchantSkill::lock(bool l)
	{
		// FUN_007f64ec: Cancel and Close wait for the answer.
		locked = l;
		Button::State state = l ? Button::State::DISABLED : Button::State::NORMAL;
		buttons[Buttons::CANCEL]->set_state(state);
		buttons[Buttons::CLOSE]->set_state(state);
	}

	void UIEnchantSkill::close()
	{
		lock(false);
		playing = false;
		UI::get().clear_tooltip(Tooltip::Parent::ENCHANTSKILL);
		deactivate();
	}

	bool UIEnchantSkill::holds_white_scroll() const
	{
		return Stage::get().get_player().get_inventory().find_item(InventoryType::Id::USE, scroll_rules::WHITE_SCROLL)
			!= 0;
	}
}
