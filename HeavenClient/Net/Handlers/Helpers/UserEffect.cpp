#include "UserEffect.h"

#include "../../Audio/Audio.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIChatBar.h"

#include <nlnx/nx.hpp>

#include <string>

namespace ms
{
	namespace user_effect
	{
		namespace
		{
			// A grey chat line (type 7), for whoever's effect it is.
			void chat_line(const std::string& line)
			{
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(line, UIChatBar::LineType::GREY);
			}

			void show_on(Optional<Char> user, nl::node effect)
			{
				if (user && effect)
					user->show_effect(Animation(effect));
			}
		}

		bool show_shared(int32_t cid, int8_t mode, InPacket& recv)
		{
			Stage& stage = Stage::get();
			Optional<Char> user = stage.get_character(cid);

			switch (mode)
			{
			case 0: // level up
				if (user)
				{
					user->show_effect_id(CharEffect::Id::LEVELUP);
					Sound(Sound::Name::LEVELUP).play();
				}

				return true;
			case 6: // a charm kept the EXP
			{
				bool safety_charm = recv.read_byte() != 0;
				uint8_t times = recv.read_byte();
				uint8_t days = recv.read_byte();

				if (safety_charm)
					chat_line("The EXP did not drop after using the Safety Charm once.(" + std::to_string(days) + " days/" + std::to_string(times) + " times left)");
				else
					chat_line("The EXP did not drop after using " + ItemData::get(recv.read_int()).get_name() + " item.");

				return true;
			}
			case 10: // a recovery's amount over the head
			{
				uint8_t amount = recv.read_byte();

				// v83 draws 0 in its other style (FUN_0092ec50); that one is not known.
				if (user && amount > 0)
					user->show_heal(amount);

				return true;
			}
			case 11: // an item's ItemEff animation and use sound
			{
				int32_t itemid = recv.read_int();

				show_on(user, nl::nx::effect["ItemEff.img"][std::to_string(itemid)]);
				Sound(itemid).play();

				return true;
			}
			case 12: // an image where the user stands; v83 delays it by a state of the user's not known
			case 18: // an animation where the user stands, as the intro cutscenes
			{
				std::string path = recv.read_string();

				if (user)
					if (nl::node effect = Stage::effect_node(path))
						stage.show_ground_effect(effect, user->get_position());

				return true;
			}
			case 15: // an item levelled up
				if (user)
				{
					user->show_effect_id(CharEffect::Id::ITEM_LEVELUP);
					Sound(Sound::Name::ITEMLEVELUP).play();
				}

				return true;
			case 16: // the Item Maker's glow and sound: 0 made it, 1 failed, anything else nothing
			{
				int32_t result = recv.read_int();

				if (result != 0 && result != 1)
					return true;

				bool made = result == 0;

				show_on(user, nl::nx::effect["BasicEff.img"]["ItemMake"][made ? "Success" : "Failure"]);
				Sound(made ? Sound::Name::ENCHANTSUCCESS : Sound::Name::ENCHANTFAILURE).play();

				return true;
			}
			case 17: // EXP gained: everyone sees it, only its owner hears it
				if (user)
				{
					user->show_effect_id(CharEffect::Id::INC_EXP);

					if (stage.is_player(cid))
						Sound(Sound::Name::INCEXP).play();
				}

				return true;
			case 21: // a Wheel of Destiny revived someone
				chat_line("You have used 1 Wheel of Destiny in order to revive at the current map. (" + std::to_string(static_cast<uint8_t>(recv.read_byte())) + " left)");

				return true;
			case 22: // an animation on the user, only in the sheep field type (0x13)
			{
				if (stage.get_mapinfo().get_fieldtype() != 0x13)
					return true;

				show_on(user, Stage::effect_node(recv.read_string()));

				return true;
			}
			case 23: // a UI image on the user, as the tutorial hints
			{
				std::string path = recv.read_string();
				recv.read_int();

				show_on(user, Stage::effect_node(path));

				return true;
			}
			case 24: // an item's use sound and an animation on the user
			{
				int32_t itemid = recv.read_int();
				std::string path = recv.read_string();

				Sound(itemid).play();
				show_on(user, Stage::effect_node(path));

				return true;
			}
			case 25: // the field plays a sound from Sound.nx
				if (nl::node sound = Stage::effect_node("Sound/" + recv.read_string()))
					Sound(sound).play();

				return true;
			case 26: // a Spirit Stone revived someone
				chat_line("You have revived on the current map through the effect of the Spirit Stone.");

				return true;
			default:
				// 19 and anything past 26 do nothing.
				return mode == 19 || static_cast<uint8_t>(mode) > 26;
			}
		}
	}
}
