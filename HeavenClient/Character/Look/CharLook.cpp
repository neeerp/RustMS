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
#include "CharLook.h"

#include "AttackStances.h"

#include "../MorphRules.h"
#include "../MountRules.h"
#include "../Pets.h"

#include "../../Data/WeaponData.h"

namespace ms
{
	CharLook::CharLook(const LookEntry &entry)
	{
		reset();

		set_body(entry.skin);
		set_hair(entry.hairid);
		set_face(entry.faceid);

		for (auto &equip : entry.equips)
		{
			// Pet equipment is the pets', not the character's.
			if (pets::is_equip_part(equip.first))
			{
				set_pet_part(equip.first, equip.second);

				continue;
			}

			add_equip(equip.second);
		}
	}

	CharLook::CharLook()
	{
		reset();

		body = nullptr;
		hair = nullptr;
		face = nullptr;
	}

	void CharLook::reset()
	{
		flip = true;

		action = nullptr;
		actionstr = "";
		actframe = 0;

		set_stance(Stance::Id::STAND1);
		stframe.set(0);
		stelapsed = 0;

		set_expression(Expression::Id::DEFAULT);
		expframe.set(0);
		expelapsed = 0;
	}

	void
	CharLook::draw(const DrawArgument &args, Stance::Id interstance, Expression::Id interexpression, uint8_t interframe,
				   uint8_t interexpframe) const
	{
		Point<int16_t> faceshift = drawinfo.getfacepos(interstance, interframe);
		DrawArgument faceargs = args + DrawArgument{faceshift, false, Point<int16_t>{}};

		if (Stance::is_climbing(interstance))
		{
			body->draw(interstance, Body::Layer::BODY, interframe, args);
			equips.draw(EquipSlot::Id::GLOVES, interstance, Clothing::Layer::GLOVE, interframe, args);
			equips.draw(EquipSlot::Id::SHOES, interstance, Clothing::Layer::SHOES, interframe, args);
			equips.draw(EquipSlot::Id::BOTTOM, interstance, Clothing::Layer::PANTS, interframe, args);
			equips.draw(EquipSlot::Id::TOP, interstance, Clothing::Layer::TOP, interframe, args);
			equips.draw(EquipSlot::Id::TOP, interstance, Clothing::Layer::MAIL, interframe, args);
			equips.draw(EquipSlot::Id::CAPE, interstance, Clothing::Layer::CAPE, interframe, args);
			body->draw(interstance, Body::Layer::HEAD, interframe, args);
			equips.draw(EquipSlot::Id::EARACC, interstance, Clothing::Layer::EARRINGS, interframe, args);

			switch (equips.getcaptype())
			{
			case CharEquips::CapType::NONE:
				hair->draw(interstance, Hair::Layer::BACK, interframe, args);
				break;
			case CharEquips::CapType::HEADBAND:
				equips.draw(EquipSlot::Id::HAT, interstance, Clothing::Layer::CAP, interframe, args);
				hair->draw(interstance, Hair::Layer::BACK, interframe, args);
				break;
			case CharEquips::CapType::HALFCOVER:
				hair->draw(interstance, Hair::Layer::BELOWCAP, interframe, args);
				equips.draw(EquipSlot::Id::HAT, interstance, Clothing::Layer::CAP, interframe, args);
				break;
			case CharEquips::CapType::FULLCOVER:
				equips.draw(EquipSlot::Id::HAT, interstance, Clothing::Layer::CAP, interframe, args);
				break;
			}

			equips.draw(EquipSlot::Id::SHIELD, interstance, Clothing::Layer::BACKSHIELD, interframe, args);
			equips.draw(EquipSlot::Id::WEAPON, interstance, Clothing::Layer::BACKWEAPON, interframe, args);
		}
		else if (interstance == Stance::Id::DEAD)
		{
			// The ghost is only a body. It wears the standing hair and head gear, moved onto its neck.
			Stance::Id standing = Stance::Id::STAND1;
			Point<int16_t> neckshift = drawinfo.get_head_position(interstance, interframe) -
									   drawinfo.get_head_position(standing, 0);
			DrawArgument headargs = args + DrawArgument{neckshift, false, Point<int16_t>{}};

			hair->draw(standing, Hair::Layer::BELOWBODY, 0, headargs);
			equips.draw(EquipSlot::Id::HAT, standing, Clothing::Layer::CAP_BELOW_BODY, 0, headargs);
			body->draw(interstance, Body::Layer::BODY, interframe, args);
			equips.draw(EquipSlot::Id::EARACC, standing, Clothing::Layer::EARRINGS, 0, headargs);
			body->draw(interstance, Body::Layer::HEAD, interframe, args);
			hair->draw(standing, Hair::Layer::SHADE, 0, headargs);
			face->draw(interexpression, interexpframe, faceargs);
			equips.draw(EquipSlot::Id::FACE, standing, Clothing::Layer::FACEACC, 0, faceargs);
			equips.draw(EquipSlot::Id::EYEACC, standing, Clothing::Layer::EYEACC, 0, headargs);
			draw_cap(headargs, standing, 0);
		}
		else
		{
			hair->draw(interstance, Hair::Layer::BELOWBODY, interframe, args);
			equips.draw(EquipSlot::Id::CAPE, interstance, Clothing::Layer::CAPE, interframe, args);
			equips.draw(EquipSlot::Id::SHIELD, interstance, Clothing::Layer::SHIELD_BELOW_BODY, interframe, args);
			equips.draw(EquipSlot::Id::WEAPON, interstance, Clothing::Layer::WEAPON_BELOW_BODY, interframe, args);
			equips.draw(EquipSlot::Id::HAT, interstance, Clothing::Layer::CAP_BELOW_BODY, interframe, args);
			body->draw(interstance, Body::Layer::BODY, interframe, args);
			equips.draw(EquipSlot::Id::GLOVES, interstance, Clothing::Layer::WRIST_OVER_BODY, interframe, args);
			equips.draw(EquipSlot::Id::GLOVES, interstance, Clothing::Layer::GLOVE_OVER_BODY, interframe, args);
			equips.draw(EquipSlot::Id::SHOES, interstance, Clothing::Layer::SHOES, interframe, args);
			body->draw(interstance, Body::Layer::ARM_BELOW_HEAD, interframe, args);

			if (equips.has_overall())
			{
				equips.draw(EquipSlot::Id::TOP, interstance, Clothing::Layer::MAIL, interframe, args);
			}
			else
			{
				equips.draw(EquipSlot::Id::BOTTOM, interstance, Clothing::Layer::PANTS, interframe, args);
				equips.draw(EquipSlot::Id::TOP, interstance, Clothing::Layer::TOP, interframe, args);
			}

			body->draw(interstance, Body::Layer::ARM_BELOW_HEAD_OVER_MAIL, interframe, args);
			equips.draw(EquipSlot::Id::SHIELD, interstance, Clothing::Layer::SHIELD_OVER_HAIR, interframe, args);
			equips.draw(EquipSlot::Id::EARACC, interstance, Clothing::Layer::EARRINGS, interframe, args);
			body->draw(interstance, Body::Layer::HEAD, interframe, args);
			hair->draw(interstance, Hair::Layer::SHADE, interframe, args);
			face->draw(interexpression, interexpframe, faceargs);
			equips.draw(EquipSlot::Id::FACE, interstance, Clothing::Layer::FACEACC, 0, faceargs);
			equips.draw(EquipSlot::Id::EYEACC, interstance, Clothing::Layer::EYEACC, interframe, args);
			equips.draw(EquipSlot::Id::SHIELD, interstance, Clothing::Layer::SHIELD, interframe, args);
			draw_cap(args, interstance, interframe);
			equips.draw(EquipSlot::Id::WEAPON, interstance, Clothing::Layer::WEAPON_BELOW_ARM, interframe, args);
			bool twohanded = is_twohanded(interstance);

			if (twohanded)
			{
				equips.draw(EquipSlot::Id::TOP, interstance, Clothing::Layer::MAILARM, interframe, args);
				body->draw(interstance, Body::Layer::ARM, interframe, args);
				equips.draw(EquipSlot::Id::WEAPON, interstance, Clothing::Layer::WEAPON, interframe, args);
			}
			else
			{
				equips.draw(EquipSlot::Id::WEAPON, interstance, Clothing::Layer::WEAPON, interframe, args);
				body->draw(interstance, Body::Layer::ARM, interframe, args);
				equips.draw(EquipSlot::Id::TOP, interstance, Clothing::Layer::MAILARM, interframe, args);
			}

			equips.draw(EquipSlot::Id::GLOVES, interstance, Clothing::Layer::WRIST, interframe, args);
			equips.draw(EquipSlot::Id::GLOVES, interstance, Clothing::Layer::GLOVE, interframe, args);
			equips.draw(EquipSlot::Id::WEAPON, interstance, Clothing::Layer::WEAPON_OVER_GLOVE, interframe, args);

			body->draw(interstance, Body::Layer::HAND_BELOW_WEAPON, interframe, args);

			body->draw(interstance, Body::Layer::ARM_OVER_HAIR, interframe, args);
			body->draw(interstance, Body::Layer::ARM_OVER_HAIR_BELOW_WEAPON, interframe, args);
			equips.draw(EquipSlot::Id::WEAPON, interstance, Clothing::Layer::WEAPON_OVER_HAND, interframe, args);
			equips.draw(EquipSlot::Id::WEAPON, interstance, Clothing::Layer::WEAPON_OVER_BODY, interframe, args);
			body->draw(interstance, Body::Layer::HAND_OVER_HAIR, interframe, args);
			body->draw(interstance, Body::Layer::HAND_OVER_WEAPON, interframe, args);

			equips.draw(EquipSlot::Id::GLOVES, interstance, Clothing::Layer::WRIST_OVER_HAIR, interframe, args);
			equips.draw(EquipSlot::Id::GLOVES, interstance, Clothing::Layer::GLOVE_OVER_HAIR, interframe, args);
		}
	}

	void CharLook::draw_cap(const DrawArgument &args, Stance::Id stance, uint8_t frame) const
	{
		switch (equips.getcaptype())
		{
		case CharEquips::CapType::NONE:
			// The hair covers the scalp, so it goes over the head and the face.
			hair->draw(stance, Hair::Layer::DEFAULT, frame, args);
			hair->draw(stance, Hair::Layer::OVERHEAD, frame, args);
			break;
		case CharEquips::CapType::HEADBAND:
			equips.draw(EquipSlot::Id::HAT, stance, Clothing::Layer::CAP, frame, args);
			hair->draw(stance, Hair::Layer::DEFAULT, frame, args);
			hair->draw(stance, Hair::Layer::OVERHEAD, frame, args);
			equips.draw(EquipSlot::Id::HAT, stance, Clothing::Layer::CAP_OVER_HAIR, frame, args);
			break;
		case CharEquips::CapType::HALFCOVER:
			hair->draw(stance, Hair::Layer::DEFAULT, frame, args);
			equips.draw(EquipSlot::Id::HAT, stance, Clothing::Layer::CAP, frame, args);
			break;
		case CharEquips::CapType::FULLCOVER:
			equips.draw(EquipSlot::Id::HAT, stance, Clothing::Layer::CAP, frame, args);
			break;
		}
	}

	void CharLook::draw(const DrawArgument &args, float alpha) const
	{
		if (!body || !hair || !face)
			return;

		Point<int16_t> acmove;

		if (action)
			acmove = action->get_move();

		DrawArgument relargs = {acmove, flip};

		Stance::Id interstance = stance.get(alpha);
		Expression::Id interexpression = expression.get(alpha);
		uint8_t interframe = stframe.get(alpha);
		uint8_t interexpframe = expframe.get(alpha);

		switch (interstance)
		{
			case Stance::Id::STAND1:
			case Stance::Id::STAND2:
				if (alerted)
					interstance = Stance::Id::ALERT;

				break;
		}

		if (morphed(interstance))
			MorphTemplate::get(morph).draw(morph_action(interstance), interframe, relargs + args);
		else if (riding(interstance))
			draw_mounted(relargs + args, interstance, interexpression, interframe, interexpframe);
		else
			draw(relargs + args, interstance, interexpression, interframe, interexpframe);
	}

	void CharLook::draw_mounted(const DrawArgument &args, Stance::Id interstance, Expression::Id interexp,
								uint8_t interframe, uint8_t interexpframe) const
	{
		std::string mobaction = mount_action(interstance);
		int32_t mountid = get_mount();
		int32_t saddleid = get_saddle();
		const TamingMob &mob = TamingMob::get(mountid);
		const TamingMob *saddle = saddleid > 0 ? &TamingMob::get(saddleid, mountid) : nullptr;

		Stance::Id pose = rider_pose(interstance);
		DrawArgument riderargs = args + DrawArgument{rider_shift(interstance, interframe), false, Point<int16_t>{}};

		// The parts behind the rider, back to front, then those in front.
		auto draw_parts = [&](int8_t from, int8_t to)
		{
			for (int8_t z = from; z >= to; z--)
			{
				mob.draw(mobaction, interframe, z, args);

				if (saddle)
					saddle->draw(mobaction, interframe, z, args);
			}
		};

		draw_parts(static_cast<int8_t>(std::size(mount::ZMAP) - 1), mount::BEHIND);
		draw(riderargs, pose, interexp, rider_frame(pose, interframe), interexpframe);
		draw_parts(mount::BEHIND - 1, 0);
	}

	bool CharLook::morphed(Stance::Id st) const
	{
		return morph > 0 && st != Stance::Id::DEAD;
	}

	std::string CharLook::morph_action(Stance::Id st) const
	{
		if (!morphact.empty())
			return morphact;

		return MorphTemplate::get(morph).action(Stance::names[st]);
	}

	bool CharLook::riding(Stance::Id st) const
	{
		return vehicle > 0 && st != Stance::Id::DEAD;
	}

	int32_t CharLook::get_mount() const
	{
		if (!mount::is_tamed_monster(vehicle))
			return vehicle;

		int32_t worn = equips.get_equip(EquipSlot::Id::TAMEDMOB);

		return worn > 0 ? worn : vehicle;
	}

	int32_t CharLook::get_saddle() const
	{
		return mount::is_tamed_monster(vehicle) ? equips.get_equip(EquipSlot::Id::SADDLE) : 0;
	}

	std::string CharLook::mount_action(Stance::Id st) const
	{
		int32_t mountid = get_mount();
		std::string mobaction = mount::action(Stance::names[st], tired, mountid);

		return TamingMob::get(mountid).count(mobaction) > 0 ? mobaction : "stand1";
	}

	Stance::Id CharLook::rider_pose(Stance::Id st) const
	{
		return Stance::is_climbing(st) ? Stance::Id::ROPE : Stance::Id::SIT;
	}

	uint8_t CharLook::rider_frame(Stance::Id pose, uint8_t frame) const
	{
		uint8_t poseframe = 0;

		for (uint8_t i = 0; i < frame; i++)
			poseframe = drawinfo.nextframe(pose, poseframe);

		return poseframe;
	}

	Point<int16_t> CharLook::rider_shift(Stance::Id st, uint8_t frame) const
	{
		Stance::Id pose = rider_pose(st);
		Point<int16_t> navel = TamingMob::get(get_mount()).navel(mount_action(st), frame);

		return navel - drawinfo.get_body_position(pose, rider_frame(pose, frame));
	}

	void CharLook::draw(Point<int16_t> position, bool flipped, Stance::Id interstance,
						Expression::Id interexpression) const
	{
		interstance = equips.adjust_stance(interstance);
		draw({position, flipped}, interstance, interexpression, 0, 0);
	}

	void CharLook::draw(const DrawArgument &args, Stance::Id interstance, Expression::Id interexpression) const
	{
		if (riding(interstance))
			draw_mounted(args, interstance, interexpression, 0, 0);
		else
			draw(args, equips.adjust_stance(interstance), interexpression, 0, 0);
	}

	bool CharLook::update(uint16_t timestep)
	{
		// The face and the alert keep the clock's time, even while the body
		// holds still on a rope or speeds through an attack.
		alerted.update();
		update_expression(Constants::TIMESTEP);

		if (timestep == 0)
		{
			stance.normalize();
			stframe.normalize();
			return false;
		}

		bool aniend = false;
		uint8_t frame = action ? actframe : stframe.get();

		if (heldframe >= 0 && frame == heldframe)
		{
			stance.normalize();
			stframe.normalize();
		} else if (action == nullptr)
		{
			uint16_t delay = get_delay(stance.get(), stframe.get());
			uint16_t delta = delay - stelapsed;

			if (timestep >= delta)
			{
				stelapsed = timestep - delta;

				uint8_t nextframe = getnextframe(stance.get(), stframe.get());
				float threshold = static_cast<float>(delta) / timestep;
				stframe.next(nextframe, threshold);

				if (stframe == 0)
				{
					aniend = true;

					// A superman's attack ends with the template's frames.
					if (!morphact.empty())
					{
						morphact = "";
						actionstr = "";
					}
				}
			} else
			{
				stance.normalize();
				stframe.normalize();

				stelapsed += timestep;
			}
		} else
		{
			// A frame of no delay passes at once.
			uint16_t delay = action->get_delay();
			uint16_t delta = delay > stelapsed ? delay - stelapsed : 0;

			if (timestep >= delta)
			{
				stelapsed = timestep - delta;
				actframe = drawinfo.next_actionframe(actionstr, actframe);

				if (actframe > 0)
				{
					action = drawinfo.get_action(actionstr, actframe);

					float threshold = static_cast<float>(delta) / timestep;
					stance.next(action->get_stance(), threshold);
					stframe.next(action->get_frame(), threshold);
				} else
				{
					aniend = true;
					action = nullptr;
					actionstr = "";
					set_stance(Stance::Id::STAND1);
				}
			} else
			{
				stance.normalize();
				stframe.normalize();

				stelapsed += timestep;
			}
		}

		return aniend;
	}

	void CharLook::update_expression(uint16_t timestep)
	{
		if (exphold > 0)
		{
			exphold -= timestep;

			if (exphold <= 0)
				set_expression(Expression::Id::DEFAULT);
		}

		uint16_t expdelay = face->get_delay(expression.get(), expframe.get());
		uint16_t expdelta = expdelay - expelapsed;

		if (timestep >= expdelta)
		{
			expelapsed = timestep - expdelta;

			uint8_t nextexpframe = face->nextframe(expression.get(), expframe.get());
			float fcthreshold = static_cast<float>(expdelta) / timestep;
			expframe.next(nextexpframe, fcthreshold);

			if (expframe == 0 && exphold <= 0)
			{
				if (expression == Expression::Id::DEFAULT)
					expression.next(Expression::Id::BLINK, fcthreshold);
				else
					expression.next(Expression::Id::DEFAULT, fcthreshold);
			}
		} else
		{
			expression.normalize();
			expframe.normalize();

			expelapsed += timestep;
		}
	}

	void CharLook::set_body(int32_t skin_id)
	{
		auto iter = bodytypes.find(skin_id);

		if (iter == bodytypes.end())
		{
			iter = bodytypes.emplace(
					std::piecewise_construct,
					std::forward_as_tuple(skin_id),
					std::forward_as_tuple(skin_id, drawinfo)
			).first;
		}

		body = &iter->second;
	}

	void CharLook::set_hair(int32_t hair_id)
	{
		auto iter = hairstyles.find(hair_id);

		if (iter == hairstyles.end())
		{
			iter = hairstyles.emplace(
					std::piecewise_construct,
					std::forward_as_tuple(hair_id),
					std::forward_as_tuple(hair_id, drawinfo)
			).first;
		}

		hair = &iter->second;
	}

	void CharLook::set_face(int32_t face_id)
	{
		auto iter = facetypes.find(face_id);

		if (iter == facetypes.end())
			iter = facetypes.emplace(face_id, face_id).first;

		face = &iter->second;
	}

	void CharLook::updatetwohanded()
	{
		Stance::Id basestance = Stance::baseof(stance.get());
		set_stance(basestance);
	}

	void CharLook::add_equip(int32_t itemid)
	{
		equips.add_equip(itemid, drawinfo);
		updatetwohanded();
	}

	void CharLook::remove_equip(EquipSlot::Id slot)
	{
		equips.remove_equip(slot);

		if (slot == EquipSlot::Id::WEAPON)
			updatetwohanded();
	}

	void CharLook::attack(bool degenerate)
	{
		attack(degenerate, get_attackstance(degenerate));
	}

	void CharLook::attack(bool degenerate, Stance::Id newstance)
	{
		int32_t weapon_id = equips.get_weapon();

		if (weapon_id <= 0)
			return;

		if (newstance == Stance::Id::SHOT)
		{
			stance.set(Stance::Id::SHOT);
			set_action("handgun");
		} else
		{
			stance.set(newstance);
			stframe.set(0);
			stelapsed = 0;
		}

		WeaponData::get(weapon_id).get_usesound(degenerate).play();
	}

	Stance::Id CharLook::get_attackstance(bool degenerate) const
	{
		int32_t weapon_id = equips.get_weapon();

		if (weapon_id <= 0)
			return Stance::Id::NONE;

		return getattackstance(WeaponData::get(weapon_id).get_attack(), degenerate);
	}

	void CharLook::attack(Stance::Id newstance)
	{
		if (action || newstance == Stance::Id::NONE)
			return;

		switch (newstance)
		{
			case Stance::Id::SHOT:
				set_action("handgun");
				break;
			default:
				set_stance(newstance);
				break;
		}
	}

	void CharLook::set_stance(Stance::Id newstance)
	{
		if (action || !morphact.empty() || newstance == Stance::Id::NONE)
			return;

		Stance::Id adjstance = equips.adjust_stance(newstance);

		if (stance != adjstance)
		{
			heldframe = -1;
			stance.set(adjstance);
			stframe.set(0);
			stelapsed = 0;
		}
	}

	void CharLook::hold(uint8_t frame)
	{
		heldframe = frame;
	}

	void CharLook::end_attack()
	{
		heldframe = -1;
		action = nullptr;
		actionstr = "";
		morphact = "";
	}

	Stance::Id CharLook::getattackstance(uint8_t attack, bool degenerate) const
	{
		if (stance == Stance::Id::PRONE)
			return Stance::Id::PRONESTAB;

		const std::vector<Stance::Id> &shots = attack_stances::get(attack, true);
		const std::vector<Stance::Id> &stances =
				!degenerate && !shots.empty() ? shots : attack_stances::get(attack, false);

		if (stances.empty())
			return Stance::Id::STAND1;

		size_t index = randomizer.next_int(stances.size());

		return stances[index];
	}

	uint16_t CharLook::get_delay(Stance::Id st, uint8_t fr) const
	{
		if (morphed(st))
			return MorphTemplate::get(morph).delay(morph_action(st), fr);

		// A rider keeps the mount's time (0x453ad1).
		if (riding(st))
			return TamingMob::get(get_mount()).delay(mount_action(st), fr);

		return drawinfo.get_delay(st, fr);
	}

	uint8_t CharLook::getnextframe(Stance::Id st, uint8_t fr) const
	{
		if (morphed(st))
		{
			uint8_t count = MorphTemplate::get(morph).count(morph_action(st));

			return fr + 1 < count ? fr + 1 : 0;
		}

		if (riding(st))
		{
			uint8_t count = TamingMob::get(get_mount()).count(mount_action(st));

			return fr + 1 < count ? fr + 1 : 0;
		}

		return drawinfo.nextframe(st, fr);
	}

	void CharLook::set_expression(Expression::Id newexpression, int32_t hold)
	{
		exphold = hold;

		if (expression != newexpression)
		{
			expression.set(newexpression);
			expframe.set(0);

			expelapsed = 0;
		}
	}

	void CharLook::set_action(const std::string &acstr)
	{
		if (acstr == actionstr || acstr == "")
			return;

		std::string attack = is_superman() ? morph::superman_action(acstr) : "";

		if (Stance::Id ac_stance = Stance::by_string(acstr))
		{
			set_stance(ac_stance);
		} else if (!attack.empty() && MorphTemplate::get(morph).count(attack) > 0)
		{
			// The superman plays it through once, at the stance it stands in.
			action = nullptr;
			actionstr = acstr;
			morphact = attack;
			heldframe = -1;
			stframe.set(0);
			stelapsed = 0;
		} else
		{
			action = drawinfo.get_action(acstr, 0);

			if (action)
			{
				actframe = 0;
				stelapsed = 0;
				actionstr = acstr;

				stance.set(action->get_stance());
				stframe.set(action->get_frame());
			}
		}
	}

	void CharLook::set_direction(bool f)
	{
		flip = f;
	}

	void CharLook::set_alerted(int64_t millis)
	{
		alerted.set_for(millis);
	}

	bool CharLook::get_alerted() const
	{
		return (bool) alerted;
	}

	bool CharLook::is_twohanded(Stance::Id st) const
	{
		switch (st)
		{
			case Stance::Id::STAND1:
			case Stance::Id::WALK1:
				return false;
			case Stance::Id::STAND2:
			case Stance::Id::WALK2:
				return true;
			default:
				return equips.is_twohanded();
		}
	}

	uint16_t CharLook::get_attackdelay(size_t no, uint8_t first_frame) const
	{
		// A superman hits when the action would have.
		if (action || !morphact.empty())
		{
			return drawinfo.get_attackdelay(actionstr, no);
		} else
		{
			uint16_t delay = 0;

			for (uint8_t frame = 0; frame < first_frame; frame++)
				delay += get_delay(stance.get(), frame);

			return delay;
		}
	}

	uint16_t CharLook::get_attack_length() const
	{
		uint16_t length = 0;

		if (action)
		{
			for (uint16_t delay : drawinfo.get_action_delays(actionstr))
				length += delay;
		} else
		{
			uint8_t frame = 0;

			do
			{
				length += get_delay(stance.get(), frame);
				frame = getnextframe(stance.get(), frame);
			} while (frame != 0);
		}

		return length;
	}

	std::vector<uint16_t> CharLook::get_action_delays(const std::string &action) const
	{
		return drawinfo.get_action_delays(action);
	}

	uint16_t CharLook::get_action_lock(const std::string &action) const
	{
		return drawinfo.get_action_lock(action);
	}

	uint8_t CharLook::get_frame() const
	{
		return stframe.get();
	}

	Stance::Id CharLook::get_stance() const
	{
		return stance.get();
	}

	std::string CharLook::get_action_name() const
	{
		return action || !morphact.empty() ? actionstr : Stance::names[stance.get()];
	}

	Point<int16_t> CharLook::get_facepos() const
	{
		Stance::Id st = stance.get();

		if (riding(st))
		{
			Stance::Id pose = rider_pose(st);

			return drawinfo.getfacepos(pose, rider_frame(pose, stframe.get())) + rider_shift(st, stframe.get());
		}

		return drawinfo.getfacepos(st, stframe.get());
	}

	const Body *CharLook::get_body() const
	{
		return body;
	}

	const Hair *CharLook::get_hair() const
	{
		return hair;
	}

	const Face *CharLook::get_face() const
	{
		return face;
	}

	int32_t CharLook::get_pet_wear(size_t index) const
	{
		return index < pet_wear.size() ? pet_wear[index] : 0;
	}

	void CharLook::set_pet_part(int16_t part, int32_t itemid)
	{
		if (itemid)
			pet_parts[part] = itemid;
		else
			pet_parts.erase(part);

		if (int8_t index = pets::wear_owner(part); index >= 0)
			pet_wear[index] = itemid;
	}

	const std::map<int16_t, int32_t> &CharLook::get_pet_parts() const
	{
		return pet_parts;
	}

	const CharEquips &CharLook::get_equips() const
	{
		return equips;
	}

	void CharLook::set_vehicle(int32_t v)
	{
		if (vehicle == v)
			return;

		vehicle = v;
		equips.set_riding(v > 0);
		stframe.set(0);
		stelapsed = 0;
	}

	int32_t CharLook::get_vehicle() const
	{
		return vehicle;
	}

	void CharLook::set_tired(bool t)
	{
		tired = t;
	}

	void CharLook::set_morph(int32_t m)
	{
		if (morph == m)
			return;

		morph = m;
		stframe.set(0);
		stelapsed = 0;

		if (!morphact.empty())
		{
			morphact = "";
			actionstr = "";
		}
	}

	int32_t CharLook::get_morph() const
	{
		return morph;
	}

	bool CharLook::is_superman() const
	{
		return morph > 0 && MorphTemplate::get(morph).is_superman();
	}

	void CharLook::init()
	{
		drawinfo.init();
	}

	BodyDrawInfo CharLook::drawinfo;
	std::unordered_map<int32_t, Hair> CharLook::hairstyles;
	std::unordered_map<int32_t, Face> CharLook::facetypes;
	std::unordered_map<int32_t, Body> CharLook::bodytypes;
}